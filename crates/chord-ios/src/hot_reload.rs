use chord_core::lock;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::compile_commands::{self, Job, value_of};
use crate::derived_data::path_hash;
use crate::issues::{BuildIssue, parse_line};
use crate::preview;
use crate::view_graph::{Graph, find, past_literal};

static SAVED: Mutex<BTreeMap<PathBuf, BTreeSet<String>>> = Mutex::new(BTreeMap::new());

fn saved_files() -> MutexGuard<'static, BTreeMap<PathBuf, BTreeSet<String>>> {
    lock(&SAVED)
}

pub(crate) fn saved(worktree: &Path, files: impl IntoIterator<Item = String>) {
    saved_files()
        .entry(worktree.to_path_buf())
        .or_default()
        .extend(files);
}

pub(crate) fn forget(worktree: &Path) {
    saved_files().remove(worktree);
}

fn reload_folder(worktree: &Path) -> Option<PathBuf> {
    preview::hot_reload_folder(worktree).map(|folder| folder.join("reload"))
}

fn launch_marker(folder: &Path) -> PathBuf {
    folder.join("launched")
}

static LAUNCHED: Mutex<BTreeMap<PathBuf, Launch>> = Mutex::new(BTreeMap::new());

struct Launch {
    dylib: PathBuf,
    stamp: Option<(u64, SystemTime)>,
    hidden: Option<Arc<HashMap<String, u64>>>,
}

struct Hidden {
    dylib: PathBuf,
    stamp: Option<(u64, SystemTime)>,
    functions: Arc<HashMap<String, u64>>,
}

impl Hidden {
    fn check(&self) -> Result<(), String> {
        if self.stamp.is_none() || file_stamp(&self.dylib) != self.stamp {
            return Err("the app was built again since the preview launched".into());
        }
        Ok(())
    }
}

fn file_stamp(dylib: &Path) -> Option<(u64, SystemTime)> {
    let meta = dylib.metadata().ok()?;
    Some((meta.len(), meta.modified().ok()?))
}

fn launched_hidden(worktree: &Path, read: bool) -> Result<Option<Hidden>, String> {
    let (dylib, stamp, functions) = match lock(&LAUNCHED).get(worktree) {
        Some(launch) => (launch.dylib.clone(), launch.stamp, launch.hidden.clone()),
        None => return Ok(None),
    };
    let functions = match functions {
        Some(functions) => functions,
        None if !read => return Ok(None),
        None => {
            let unread = Hidden {
                dylib,
                stamp,
                functions: Arc::default(),
            };
            unread.check()?;
            let functions = Arc::new(hidden_functions(&run(&tools().nm, &["-U"], &unread.dylib)?));
            if let Some(launch) = lock(&LAUNCHED).get_mut(worktree)
                && launch.dylib == unread.dylib
                && launch.stamp == stamp
            {
                launch.hidden = Some(functions.clone());
            }
            return Ok(Some(Hidden {
                functions,
                ..unread
            }));
        }
    };
    Ok(Some(Hidden {
        dylib,
        stamp,
        functions,
    }))
}

fn hidden_functions(nm: &str) -> HashMap<String, u64> {
    nm.lines()
        .filter_map(|line| {
            let mut fields = line.splitn(3, ' ');
            let address = fields.next()?;
            (fields.next()? == "t").then_some(())?;
            let name = fields.next()?;
            Some((name.to_string(), u64::from_str_radix(address, 16).ok()?))
        })
        .collect()
}

pub(crate) fn launched(worktree: &Path, dylib: &Path) {
    forget(worktree);
    lock(&LAUNCHED).insert(
        worktree.to_path_buf(),
        Launch {
            dylib: dylib.to_path_buf(),
            stamp: file_stamp(dylib),
            hidden: None,
        },
    );
    if let Some(folder) = reload_folder(worktree) {
        let _ = fs::remove_dir_all(&folder);
        let _ = fs::create_dir_all(&folder).and_then(|()| fs::File::create(launch_marker(&folder)));
    }
}

pub(crate) fn reload(worktree: &Path) -> Result<Option<BuildIssue>, String> {
    let saved = saved_files().get(worktree).cloned().unwrap_or_default();
    if saved.is_empty() {
        return Ok(None);
    }
    let file_id = preview::shown_file(worktree)?;
    let (module, name) = file_id.split_once('/').ok_or("the preview has no file")?;
    let drivers = compile_commands::drivers(worktree);
    let (driver, files, shown) = drivers
        .iter()
        .filter(|driver| driver.module() == Some(module))
        .find_map(|driver| {
            let files = driver.files();
            let shown = files.iter().find(|file| file_name(file) == name)?.clone();
            Some((driver, files, shown))
        })
        .ok_or_else(|| format!("{file_id} has no compile command"))?;
    let folder = reload_folder(worktree).ok_or("Chord has no cache folder")?;
    let jobs = compile_commands::jobs(worktree, driver, &files, &launch_marker(&folder))?;
    let hidden = launched_hidden(worktree, false)?;
    match compile(
        &jobs,
        &files,
        &folder,
        worktree,
        &shown,
        &saved,
        hidden.as_ref(),
    )? {
        Built::Unused => Ok(None),
        Built::Errors(issue) => Ok(Some(issue)),
        Built::Dylib { dylib, objects } => {
            if let Err(error) = preview::load(worktree, &dylib) {
                if hidden.is_some() || !error.contains("symbol not found") {
                    return Err(error);
                }
                let Some(hidden) = launched_hidden(worktree, true)? else {
                    return Err(error);
                };
                let job = jobs
                    .get(&shown)
                    .ok_or("the preview's file has no compile command")?;
                let inputs: Vec<PathBuf> =
                    objects.iter().map(|(object, _)| object.clone()).collect();
                preview::load(
                    worktree,
                    &link(job, &folder, worktree, &inputs, Some(&hidden))?,
                )?;
            }
            for (object, last) in objects {
                let _ = fs::rename(object, last);
            }
            Ok(None)
        }
    }
}

enum Built {
    Dylib {
        dylib: PathBuf,
        objects: Vec<(PathBuf, PathBuf)>,
    },
    Errors(BuildIssue),
    Unused,
}

fn compile(
    jobs: &HashMap<String, Job>,
    module: &[String],
    folder: &Path,
    worktree: &Path,
    shown: &str,
    saved: &BTreeSet<String>,
    hidden: Option<&Hidden>,
) -> Result<Built, String> {
    let others: Vec<&String> = saved.iter().filter(|file| *file != shown).collect();
    let sources: Vec<(String, String)> = if others.is_empty() {
        Vec::new()
    } else {
        module
            .iter()
            .filter_map(|file| Some((file.clone(), fs::read_to_string(file).ok()?)))
            .collect()
    };
    let graph = Graph::new(&sources);
    let mut files = BTreeSet::from([shown.to_string()]);
    for other in others {
        let name = file_name(other);
        if !Path::new(other).exists() {
            return Err(format!("{name} was deleted"));
        }
        if !module.contains(other) {
            return Err(format!(
                "{name} is new or in another module than the preview"
            ));
        }
        if !graph.is_view(other) {
            return Err(format!("{name} holds no view"));
        }
        if let Some(path) = graph.path(shown, other) {
            files.extend(path);
        }
    }
    let changed: Vec<(&String, PathBuf, PathBuf)> = files
        .iter()
        .filter(|file| saved.contains(*file))
        .map(|file| {
            let last = folder.join(format!("last-{}.o", path_hash(file)));
            let old = if last.exists() {
                last.clone()
            } else {
                PathBuf::from(jobs.get(file).map_or("", |job| job.object.as_str()))
            };
            (file, old, last)
        })
        .collect();
    if changed.is_empty() {
        return Ok(Built::Unused);
    }
    for (file, old, last) in &changed {
        if old != last && modified(old) > modified(&launch_marker(folder)) {
            return Err(format!(
                "a build after the launch recompiled {}",
                file_name(file)
            ));
        }
    }

    for entry in fs::read_dir(folder).into_iter().flatten().flatten() {
        if entry.file_name().to_string_lossy().starts_with("reload-") {
            let _ = fs::remove_file(entry.path());
        }
    }
    fs::create_dir_all(folder.join("stripped")).map_err(|error| error.to_string())?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_micros())
        .unwrap_or_default();
    let list: Vec<&String> = files.iter().collect();
    let objects: Vec<PathBuf> = list
        .iter()
        .map(|file| folder.join(format!("reload-{stamp}-{}.o", path_hash(file))))
        .collect();
    let next = AtomicUsize::new(0);
    let workers = std::thread::available_parallelism()
        .map_or(4, |count| count.get())
        .min(list.len());
    let (mut results, old_layouts) = std::thread::scope(|scope| {
        let old_layouts = scope.spawn(|| {
            changed
                .iter()
                .map(|(_, old, _)| layout(old))
                .collect::<Result<Vec<_>, _>>()
        });
        let workers: Vec<_> = (0..workers)
            .map(|_| {
                scope.spawn(|| {
                    let mut done = Vec::new();
                    loop {
                        let k = next.fetch_add(1, Ordering::Relaxed);
                        let Some(file) = list.get(k) else { break done };
                        let keep_previews = file.as_str() == shown;
                        done.push((
                            k,
                            compile_file(jobs, folder, worktree, file, keep_previews, &objects[k]),
                        ));
                    }
                })
            })
            .collect();
        let results: Vec<_> = workers
            .into_iter()
            .flat_map(|worker| worker.join().unwrap_or_default())
            .collect();
        let old_layouts = old_layouts
            .join()
            .unwrap_or_else(|_| Err("the layout check crashed".into()));
        (results, old_layouts)
    });
    if results.len() != list.len() {
        return Err("a recompile crashed".into());
    }
    results.sort_by_key(|result| result.0);
    let mut first = None;
    for (_, result) in results {
        if let Some(issue) = result? {
            first.get_or_insert(issue);
        }
    }
    if let Some(issue) = first {
        return Ok(Built::Errors(issue));
    }
    for ((file, ..), old) in changed.iter().zip(old_layouts?) {
        let k = list
            .iter()
            .position(|listed| listed == file)
            .unwrap_or_default();
        if old != layout(&objects[k])? {
            return Err(format!(
                "a stored property or enum case in {} changed",
                file_name(file)
            ));
        }
    }

    let job = jobs
        .get(shown)
        .ok_or("the preview's file has no compile command")?;
    let dylib = link(job, folder, worktree, &objects, hidden)?;
    let objects = list
        .iter()
        .zip(objects)
        .map(|(file, object)| (object, folder.join(format!("last-{}.o", path_hash(file)))))
        .collect();
    Ok(Built::Dylib { dylib, objects })
}

fn link(
    job: &Job,
    folder: &Path,
    worktree: &Path,
    objects: &[PathBuf],
    hidden: Option<&Hidden>,
) -> Result<PathBuf, String> {
    let target = value_of(&job.args, "-target").ok_or("the compile command has no -target")?;
    let sdk = value_of(&job.args, "-sdk").ok_or("the compile command has no -sdk")?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_micros())
        .unwrap_or_default();
    let mut inputs = objects.to_vec();
    if let Some(hidden) = hidden {
        let calls = hidden_calls(objects, &hidden.functions)?;
        if !calls.is_empty() {
            hidden.check()?;
            let image = file_name(&hidden.dylib.to_string_lossy());
            let slide = preview::slide(worktree, &image)?;
            let stubs = folder.join(format!("reload-{stamp}-stubs.s"));
            fs::write(&stubs, trampolines(&calls, slide)).map_err(|error| error.to_string())?;
            let object = stubs.with_extension("o");
            let output = Command::new(&tools().clang)
                .args(["-c", "-target", target, "-isysroot", sdk, "-o"])
                .arg(&object)
                .arg(&stubs)
                .output()
                .map_err(|error| format!("clang: {error}"))?;
            if !output.status.success() {
                let errors = String::from_utf8_lossy(&output.stderr);
                return Err(format!(
                    "assembling the trampolines failed: {}",
                    errors.lines().next().unwrap_or_default()
                ));
            }
            inputs.push(object);
        }
    }
    let dylib = folder.join(format!("reload-{stamp}.dylib"));
    let output = Command::new(&tools().clang)
        .args([
            "-dynamiclib",
            "-undefined",
            "dynamic_lookup",
            "-Wl,-ignore_auto_link",
            "-target",
            target,
            "-isysroot",
            sdk,
            "-o",
        ])
        .arg(&dylib)
        .args(&inputs)
        .output()
        .map_err(|error| format!("clang: {error}"))?;
    if !output.status.success() {
        let errors = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "linking failed: {}",
            errors.lines().next().unwrap_or_default()
        ));
    }
    Ok(dylib)
}

fn hidden_calls(
    objects: &[PathBuf],
    functions: &HashMap<String, u64>,
) -> Result<BTreeMap<String, u64>, String> {
    let output = Command::new(&tools().nm)
        .args(objects)
        .output()
        .map_err(|error| format!("nm: {error}"))?;
    if !output.status.success() {
        return Err("nm failed on the reload's objects".into());
    }
    let (undefined, defined) = undefined_and_defined(&String::from_utf8_lossy(&output.stdout));
    Ok(undefined
        .difference(&defined)
        .filter_map(|name| Some((name.clone(), *functions.get(name)?)))
        .collect())
}

fn undefined_and_defined(nm: &str) -> (BTreeSet<String>, BTreeSet<String>) {
    let (mut undefined, mut defined) = (BTreeSet::new(), BTreeSet::new());
    for line in nm.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        match fields[..] {
            ["U", name] => undefined.insert(name.to_string()),
            [_, _, name] => defined.insert(name.to_string()),
            _ => false,
        };
    }
    (undefined, defined)
}

fn trampolines(calls: &BTreeMap<String, u64>, slide: u64) -> String {
    let mut source = String::from(".text\n");
    for (name, address) in calls {
        source.push_str(&format!(
            ".globl \"{name}\"\n.p2align 2\n\"{name}\":\n  ldr x16, 1f\n  br x16\n  .p2align 3\n1: .quad {:#x}\n",
            address.wrapping_add(slide)
        ));
    }
    source
}

fn compile_file(
    jobs: &HashMap<String, Job>,
    folder: &Path,
    worktree: &Path,
    path: &str,
    keep_previews: bool,
    out: &Path,
) -> Result<Option<BuildIssue>, String> {
    let name = file_name(path);
    if !Path::new(path).exists() {
        return Err(format!("{name} was deleted"));
    }
    let job = jobs
        .get(path)
        .ok_or_else(|| format!("{name} has no compile command"))?;
    let stripped_folder = folder.join("stripped");
    let caches = compile_commands::caches(&job.args);
    let original = if caches {
        value_of(&job.args, "-primary-file").unwrap_or(path)
    } else {
        path
    };
    let primary = if keep_previews || caches {
        original.to_string()
    } else {
        strip_to(
            &folder.join("primary").join(path_hash(path)).join(&name),
            path,
        )
    };
    let recompile = |strip: bool| {
        let args = if strip {
            rewrite(
                &job.args,
                &primary,
                |file| stripped(&stripped_folder, file),
                out,
            )
        } else {
            rewrite(&job.args, original, str::to_string, out)
        };
        let output = Command::new(&args[0])
            .args(&args[1..])
            .current_dir(worktree)
            .output()
            .map_err(|error| format!("swift-frontend: {error}"))?;
        let errors = String::from_utf8_lossy(&output.stderr);
        let yours = errors
            .lines()
            .filter(|line| line.starts_with(path) || line.starts_with(primary.as_str()))
            .find_map(parse_line);
        Ok::<_, String>((
            output.status.success(),
            yours,
            errors.lines().next().unwrap_or_default().to_string(),
        ))
    };
    let mut result = recompile(!caches)?;
    if !caches && !result.0 && (result.1.is_none() || primary != path) {
        result = recompile(false)?;
    }
    match result {
        (true, ..) => Ok(None),
        (false, Some(issue), _) => Ok(Some(issue)),
        (false, None, first) => Err(format!("recompiling {name} failed: {first}")),
    }
}

const DROP_WITH_VALUE: &[&str] = &[
    "-o",
    "-emit-dependencies-path",
    "-serialize-diagnostics-path",
    "-emit-reference-dependencies-path",
    "-emit-module-path",
    "-emit-module-doc-path",
    "-emit-module-source-info-path",
    "-emit-abi-descriptor-path",
    "-index-unit-output-path",
    "-index-store-path",
    "-supplementary-output-file-map",
    "-emit-const-values-path",
    "-pch-output-dir",
    "-emit-localized-strings-path",
];
const DROP: &[&str] = &[
    "-index-system-modules",
    "-index-ignore-system-modules",
    "-profile-generate",
    "-profile-coverage-mapping",
    "-emit-localized-strings",
];

fn rewrite(
    args: &[String],
    primary: &str,
    input: impl Fn(&str) -> String,
    out: &Path,
) -> Vec<String> {
    let mut rewritten = Vec::new();
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-primary-file" => {
                args.next();
                rewritten.extend(["-primary-file".to_string(), primary.to_string()]);
            }
            flag if DROP_WITH_VALUE.contains(&flag) => drop(args.next()),
            flag if DROP.contains(&flag) => {}
            file if file.ends_with(".swift") && !file.starts_with('-') => {
                rewritten.push(input(file))
            }
            _ => rewritten.push(arg.clone()),
        }
    }
    rewritten.extend(["-o".to_string(), out.to_string_lossy().into_owned()]);
    rewritten
}

fn strip_previews(source: &str) -> String {
    let b = source.as_bytes();
    let n = b.len();
    let (mut out, mut i) = (Vec::with_capacity(n), 0);
    while let Some(j) = find(b, i, b"#Preview") {
        let line = b[..j]
            .iter()
            .rposition(|&c| c == b'\n')
            .map_or(0, |k| k + 1);
        if !b[line..j].iter().all(u8::is_ascii_whitespace) {
            out.extend_from_slice(&b[i..j + 8]);
            i = j + 8;
            continue;
        }
        out.extend_from_slice(&b[i..j]);
        let (mut k, mut depth, mut closed) = (j + 8, 0, false);
        while k < n {
            if let Some(end) = past_literal(b, k) {
                k = end;
            } else {
                match b[k] {
                    b'{' | b'(' => depth += 1,
                    b'}' | b')' => {
                        depth -= 1;
                        if depth == 0
                            && b[k + 1..].iter().find(|c| !c.is_ascii_whitespace()) != Some(&b'{')
                        {
                            k += 1;
                            closed = true;
                            break;
                        }
                        if depth < 0 {
                            break;
                        }
                    }
                    _ => {}
                }
                k += 1;
            }
        }
        if !closed {
            return source.into();
        }
        i = k;
    }
    out.extend_from_slice(&b[i..]);
    String::from_utf8_lossy(&out).into_owned()
}

fn file_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn strip_to(copy: &Path, path: &str) -> String {
    if modified(copy)
        .is_some_and(|copied| modified(Path::new(path)).is_some_and(|source| copied >= source))
    {
        return copy.to_string_lossy().into_owned();
    }
    let Ok(source) = fs::read_to_string(path) else {
        return path.into();
    };
    if !source.contains("#Preview") {
        return path.into();
    }
    static N: AtomicUsize = AtomicUsize::new(0);
    let partial = copy.with_extension(format!("{}.partial", N.fetch_add(1, Ordering::Relaxed)));
    let written = copy
        .parent()
        .is_some_and(|parent| fs::create_dir_all(parent).is_ok())
        && fs::write(&partial, strip_previews(&source)).is_ok()
        && fs::rename(&partial, copy).is_ok();
    if written {
        copy.to_string_lossy().into_owned()
    } else {
        path.into()
    }
}

fn stripped(folder: &Path, path: &str) -> String {
    strip_to(
        &folder.join(format!("{}-{}", path_hash(path), file_name(path))),
        path,
    )
}

fn modified(path: &Path) -> Option<SystemTime> {
    path.metadata().and_then(|meta| meta.modified()).ok()
}

fn layout_symbols(nm: &str) -> BTreeSet<&str> {
    nm.lines()
        .filter_map(|line| line.split_whitespace().last())
        .filter(|symbol| {
            symbol.ends_with("Wvd") || (symbol.ends_with("MV") && !symbol.ends_with("ZMV"))
        })
        .collect()
}

fn run(tool: &str, args: &[&str], object: &Path) -> Result<String, String> {
    let output = Command::new(tool)
        .args(args)
        .arg(object)
        .output()
        .map_err(|error| format!("{tool}: {error}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(format!("{tool} failed on {}", object.display()))
    }
}

fn layout(object: &Path) -> Result<(BTreeSet<String>, String), String> {
    let nm = run(&tools().nm, &["-U"], object)?;
    let names = run(
        &tools().otool,
        &["-X", "-s", "__TEXT", "__swift5_reflstr"],
        object,
    )?;
    let names = names
        .lines()
        .flat_map(|line| line.split_whitespace().skip(1))
        .collect();
    Ok((
        layout_symbols(&nm).into_iter().map(String::from).collect(),
        names,
    ))
}

struct Tools {
    clang: String,
    nm: String,
    otool: String,
}

fn tools() -> &'static Tools {
    static TOOLS: OnceLock<Tools> = OnceLock::new();
    TOOLS.get_or_init(|| {
        let find = |name: &str| {
            crate::process::xcrun(&["-f", name], &|_| {})
                .map_or_else(|_| name.into(), |path| path.trim().to_string())
        };
        Tools {
            clang: find("clang"),
            nm: find("nm"),
            otool: find("otool"),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrite_compiles_the_primary_alone_without_build_outputs() {
        let args: Vec<String> = "/X/swift-frontend -frontend -c -primary-file /W/A.swift /W/B.swift -emit-dependencies-path /DD/A.d -index-store-path /DD/Index -index-system-modules -target arm64-apple-ios17.0-simulator -module-name Lib -o /DD/A.o"
            .split(' ')
            .map(String::from)
            .collect();
        let rewritten = rewrite(
            &args,
            "/P/A.swift",
            |file| format!("S{file}"),
            Path::new("/t/r.o"),
        );
        assert_eq!(
            rewritten.join(" "),
            "/X/swift-frontend -frontend -c -primary-file /P/A.swift S/W/B.swift -target arm64-apple-ios17.0-simulator -module-name Lib -o /t/r.o"
        );
    }

    #[test]
    fn strips_top_level_previews_only() {
        let source = "struct A: View {\n  var body: some View { Text(\"}\") }\n}\n\n#Preview(\"Dark\") {\n  A() // }\n    .padding() /* { */\n  Text(\"\"\"\n  }\n  \"\"\")\n}\nlet x = \"#Preview\"\n  #Preview { B { \"😀\\\"\" } }\nstruct C {}\n";
        assert_eq!(
            strip_previews(source),
            "struct A: View {\n  var body: some View { Text(\"}\") }\n}\n\n\nlet x = \"#Preview\"\n  \nstruct C {}\n"
        );
        assert_eq!(
            strip_previews("a\n#Preview(\"x\", body: { A() })\nb"),
            "a\n\nb"
        );
        assert_eq!(
            strip_previews("a\n#Preview { Text(#\"\\\"#) /* { /* } */ */ }\nb"),
            "a\n\nb"
        );
        let open = "a\n#Preview { Text(\"x\")\nb";
        assert_eq!(strip_previews(open), open);
    }

    #[test]
    fn reads_hidden_functions_and_the_calls_objects_leave_open() {
        let dylib = "0000000000001a00 t _$s2DS6ButtonV5titleACSS_SbtcfcfA_\n0000000000002000 T _$s2DS6ButtonV4bodyQrvg\n0000000000003000 s _$s2DS4dataSivp\n0000000000004000 t -[Foo bar:]\n";
        assert_eq!(
            hidden_functions(dylib),
            HashMap::from([
                ("_$s2DS6ButtonV5titleACSS_SbtcfcfA_".to_string(), 0x1a00),
                ("-[Foo bar:]".to_string(), 0x4000),
            ])
        );
        let objects = "\n/r/a.o:\n                 U _$sA\n                 U _$sB\n0000000000000010 T _$sC\n\n/r/b.o:\n0000000000000020 t _$sB\n";
        let (undefined, defined) = undefined_and_defined(objects);
        assert_eq!(undefined.difference(&defined).collect::<Vec<_>>(), ["_$sA"]);
    }

    #[test]
    fn stored_property_changes_show_in_layout_symbols() {
        let old = "0000000000000010 S _$s5MyApp4ItemC5levelSivpWvd\n0000000000000020 T _$s5MyApp4ItemC4bodyQrvg\n0000000000000030 S _$s5MyApp4ItemV4nameSSvpMV\n";
        let same = "0000000000000090 T _$s5MyApp4ItemC4bodyQrvg\n0000000000000010 S _$s5MyApp4ItemC5levelSivpWvd\n0000000000000030 S _$s5MyApp4ItemV4nameSSvpMV\n";
        let added = format!("{same}0000000000000040 S _$s5MyApp4ItemC4noteSSvpWvd\n");
        let line_moved = format!(
            "{same}0000000000000050 S _$s5MyApp0021ListScreenswift_IfFDefMX410_0_33_0123456789ABCDEF0123456789ABCDEFLl7PreviewfMf_15PreviewRegistryfMu_V4lineSivpZMV\n"
        );
        assert_eq!(layout_symbols(old), layout_symbols(same));
        assert_eq!(layout_symbols(old), layout_symbols(&line_moved));
        assert_ne!(layout_symbols(old), layout_symbols(&added));
    }
}
