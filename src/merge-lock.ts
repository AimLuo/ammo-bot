export type MergeLock = {
  acquire(key: string): Promise<() => void>;
};

export function createMergeLock(): MergeLock {
  const tails = new Map<string, Promise<void>>();

  return {
    async acquire(key: string) {
      let release!: () => void;
      const held = new Promise<void>((resolve) => {
        release = resolve;
      });
      const previous = tails.get(key) ?? Promise.resolve();
      tails.set(
        key,
        previous.then(() => held),
      );
      await previous;
      return () => {
        release();
      };
    },
  };
}

export const mergeLock = createMergeLock();
