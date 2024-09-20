import { defineConfig } from "cypress";
import sqlite from "sqlite3";
import users from "./cypress/fixtures/users.json";

export default defineConfig({
  e2e: {
    baseUrl: "http://localhost:3000",
    setupNodeEvents(on) {
      on("task", {
        resetDatabase() {
          const db_url = process.env.DB;
          if (!db_url) {
            throw new Error("Missing 'DB' environment variable");
          }

          const db = new sqlite.Database(db_url);

          db.serialize(() => {
            db.run("DELETE FROM uploads");
            db.run("DELETE FROM users");
          });

          db.close();
          return null;
        },

        initialUsers() {
          const db_url = process.env.DB;
          if (!db_url) {
            throw new Error("Missing 'DB' environment variable");
          }

          const db = new sqlite.Database(db_url);
          db.serialize(() => {
            console.log("Deleting all users and uploads");
            db.run("DELETE FROM uploads");
            db.run("DELETE FROM users");

            const stmt = db.prepare(
              "INSERT INTO users (username, name, password, enabled, admin, created_at) VALUES (?, ?, ?, 1, ?, ?)",
            );

            console.log("Inserting initial users");
            stmt.run(
              users.admin.username,
              users.admin.name,
              users.admin.passwordHash,
              true,
              new Date().toISOString(),
            );

            stmt.run(
              users.user.username,
              users.user.name,
              users.user.passwordHash,
              false,
              new Date().toISOString(),
            );

            stmt.finalize();
          });

          db.close();
          console.log("Initial users inserted");
          return null;
        },
      });
    },
  },
});
