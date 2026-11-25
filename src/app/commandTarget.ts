export const onTarget = <T>(find: () => T | null | undefined, act: (target: T) => void) => ({
  when: () => find() != null,
  run: () => {
    const target = find();
    if (target != null) act(target);
  },
});
