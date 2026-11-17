// A command's condition and action on one target, found once for each: the command runs while `find` finds a target, and acts on that one.
// Apart from commands.ts, which imports every feature's commands, so a feature's table can call it while it loads.

export const onTarget = <T>(find: () => T | null | undefined, act: (target: T) => void) => ({
  when: () => find() != null,
  run: () => {
    const target = find();
    if (target != null) act(target);
  },
});
