import { test } from "node:test";
import assert from "node:assert/strict";
import { branchName, goneWorktrees, moved, noProjects, withFirstWorktreeSelected, withProject, withoutProject, withoutWorktree, withSavedSelection, withSelectionRestored, withWorktrees } from "./projects.ts";

const tree = (path: string, branch: string) => ({ path, branch, xcode: false, git: true });

test("removing a project clears the selection only when it held the selected worktree", () => {
  let projects = withProject(withProject(noProjects, { path: "/a", name: "a" }), { path: "/b", name: "b" });
  projects = withWorktrees(projects, "/a", [tree("/a-fix", "fix")]);
  projects = withWorktrees(projects, "/b", [tree("/b", "main")]);

  assert.equal(withoutProject({ ...projects, selected: "/a-fix" }, "/a").selected, null);
  assert.equal(withoutProject({ ...projects, selected: "/b" }, "/a").selected, "/b");
});

test("an added project opens its first worktree only while nothing is open", () => {
  let projects = withProject(noProjects, { path: "/a", name: "a" });
  projects = withWorktrees(projects, "/a", [tree("/a", "main"), tree("/a-fix", "fix")]);

  assert.equal(withFirstWorktreeSelected(projects, "/a").selected, "/a");
  assert.equal(withFirstWorktreeSelected({ ...projects, selected: "/b" }, "/a").selected, "/b");
});

test("a typed branch name has its whitespace runs turned into dashes and its ends trimmed", () => {
  assert.equal(branchName("  feature  my x\t"), "feature-my-x");
  assert.equal(branchName("   "), "");
});

test("deleting the selected worktree selects the one below it, or above when it was last", () => {
  const list = [
    tree("/a", "main"),
    tree("/a-one", "one"),
    tree("/a-two", "two"),
  ];
  const projects = withWorktrees(withProject(noProjects, { path: "/a", name: "a" }), "/a", list);

  assert.equal(withoutWorktree({ ...projects, selected: "/a-one" }, "/a", "/a-one").selected, "/a-two");
  assert.equal(withoutWorktree({ ...projects, selected: "/a-two" }, "/a", "/a-two").selected, "/a-one");
  assert.equal(withoutWorktree({ ...projects, selected: "/a" }, "/a", "/a-two").selected, "/a");
});

test("a refused delete gives the selection back only when the delete had moved it", () => {
  const projects = { ...noProjects, selected: "/a-two" };
  assert.equal(withSelectionRestored(projects, "/a-one", "/a-one", "/a-two").selected, "/a-one");
  assert.equal(withSelectionRestored(projects, "/a-one", "/a", "/a-two").selected, "/a-two");
  assert.equal(withSelectionRestored({ ...projects, selected: "/a" }, "/a-one", "/a-one", "/a-two").selected, "/a");
});

test("the saved worktree is selected only if it still exists and nothing is selected yet", () => {
  const projects = withWorktrees(withProject(noProjects, { path: "/a", name: "a" }), "/a", [tree("/a-one", "one")]);

  assert.equal(withSavedSelection(projects, "/a-one").selected, "/a-one");
  assert.equal(withSavedSelection(projects, "/gone").selected, null);
  assert.equal(withSavedSelection({ ...projects, selected: "/a" }, "/a-one").selected, "/a");
});

test("a project's worktrees missing from its new list are gone", () => {
  const list = [
    tree("/a", "main"),
    tree("/a-one", "one"),
  ];
  const projects = withWorktrees(withProject(noProjects, { path: "/a", name: "a" }), "/a", list);

  assert.deepEqual(goneWorktrees(projects, "/a", [tree("/a", "main")]), ["/a-one"]);
  assert.deepEqual(goneWorktrees(projects, "/a", []), ["/a", "/a-one"]);
});

test("a moved item lands before the item at its drop slot, counted in the list as it was", () => {
  assert.deepEqual(moved(["a", "b", "c", "d"], 0, 3), ["b", "c", "a", "d"]);
  assert.deepEqual(moved(["a", "b", "c", "d"], 3, 1), ["a", "d", "b", "c"]);
  assert.deepEqual(moved(["a", "b", "c", "d"], 1, 4), ["a", "c", "d", "b"]);
});
