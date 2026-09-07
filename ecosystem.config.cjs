/**
 * PM2 process file for Ammo-bot.
 *
 * One fork only — scanner and merge lock assume a single process.
 * No max_memory_restart and no Node heap cap: pipelines spawned here
 * may use the host freely.
 *
 *   bun run build && pm2 start ecosystem.config.cjs
 */
module.exports = {
  apps: [
    {
      name: "ammo-bot",
      cwd: __dirname,
      script: "scripts/serve.mjs",
      interpreter: "node",
      exec_mode: "fork",
      instances: 1,
      env: {
        NODE_ENV: "production",
        PORT: "20000",
        HOST: "0.0.0.0",
      },
    },
  ],
};
