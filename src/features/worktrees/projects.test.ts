// Tests for the project list rules.
import { test } from "node:test";
import assert from "node:assert/strict";
import { branchName, noProjects, withProject, withoutProject, withoutWorktree, withSelectionRestored, withWorktrees } from "./projects.ts";

test("removing a project clears the selection only when it held the selected worktree", () => {
  let projects = withProject(withProject(noProjects, { path: "/a", name: "a" }), { path: "/b", name: "b" });
  projects = withWorktrees(projects, "/a", [{ path: "/a-fix", branch: "fix" }]);
  projects = withWorktrees(projects, "/b", [{ path: "/b", branch: "main" }]);

  assert.equal(withoutProject({ ...projects, selected: "/a-fix" }, "/a").selected, null);
  assert.equal(withoutProject({ ...projects, selected: "/b" }, "/a").selected, "/b");
});

test("a typed branch name has its whitespace runs turned into dashes and its ends trimmed", () => {
  assert.equal(branchName("  feature  my x\t"), "feature-my-x");
  assert.equal(branchName("   "), "");
});

test("deleting the selected worktree selects the one below it, or above when it was last", () => {
  const list = [
    { path: "/a", branch: "main" },
    { path: "/a-one", branch: "one" },
    { path: "/a-two", branch: "two" },
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
