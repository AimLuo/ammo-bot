// PM2 process file for the Next.js dev server.
// From this directory: pm2 start ecosystem.config.cjs
// The Rust API must already be running (`markplane serve --dev` on :4200).

const path = require("path");

module.exports = {
  apps: [
    {
      name: "markplane-ui",
      cwd: __dirname,
      script: path.join(__dirname, "node_modules/next/dist/bin/next"),
      args: "dev --port 4201",
      interpreter: "node",
      exec_mode: "fork",
      instances: 1,
      autorestart: true,
      watch: false,
      max_memory_restart: "1G",
      exp_backoff_restart_delay: 100,
      kill_timeout: 5000,
      merge_logs: true,
      time: true,
      error_file: path.join(__dirname, "logs/pm2-error.log"),
      out_file: path.join(__dirname, "logs/pm2-out.log"),
      env: {
        NODE_ENV: "development",
      },
    },
  ],
};
