import { serve } from "@hono/node-server";
import { logger } from "hono/logger";

import { app } from "./routes.ts";

app.use("*", logger());

const PORT = Number(process.env.PORT ?? 8080);

serve({ fetch: app.fetch, port: PORT }, (info) => {
  console.log(`[iap-backend] listening on http://localhost:${info.port}`);
  console.log(`  POST /iap/grant       — verify JWS + grant credits`);
  console.log(`  POST /apple/webhook   — App Store Server Notifications V2`);
  console.log(`  GET  /balance/:wallet — read user balance`);
});
