// Tests for the project list rules.
import { test } from "node:test";
import assert from "node:assert/strict";
import { noProjects, withProject, withoutProject, withWorktrees } from "./projects.ts";

test("removing a project clears the selection only when it held the selected worktree", () => {
  let projects = withProject(withProject(noProjects, { path: "/a", name: "a" }), { path: "/b", name: "b" });
  projects = withWorktrees(projects, "/a", [{ path: "/a-fix", branch: "fix" }]);
  projects = withWorktrees(projects, "/b", [{ path: "/b", branch: "main" }]);

  assert.equal(withoutProject({ ...projects, selected: "/a-fix" }, "/a").selected, null);
  assert.equal(withoutProject({ ...projects, selected: "/b" }, "/a").selected, "/b");
});
