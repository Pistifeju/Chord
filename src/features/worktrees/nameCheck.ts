export function nameCheck() {
  let latest = 0;
  return {
    drop: () => void latest++,
    async run(ask: () => Promise<string | null>, land: (problem: string | null) => void) {
      const id = ++latest;
      let problem: string | null;
      try {
        problem = await ask();
      } catch (error) {
        problem = String(error);
      }
      if (id === latest) land(problem);
    },
  };
}
