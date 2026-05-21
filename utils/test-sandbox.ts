import { Command } from "commander";
import { Client, Collection, Dataset } from "topk-js";
import Bun from "bun";
import { z } from "zod";
import pino from "pino";
import pretty from "pino-pretty";
import pLimit, { LimitFunction } from "p-limit";
import pRetry, { Options as RetryOptions } from "p-retry";
import ms, { StringValue } from "ms";

type Env = ReturnType<typeof loadEnv>;
type State = { datasets: Dataset[]; collections: Collection[] };
type Opts = {
  concurrency: number;
  prefix?: string;
  age?: number;
  timeout: number;
  dryRun: boolean;
};

const log = pino({ level: process.env.LOG_LEVEL ?? "info" }, pretty());
const program = new Command();

program
  .name("test-sandbox")
  .description("Run a command with sandbox discovery before and after")
  .argument("[cmd...]", "command to run")
  .option(
    "--concurrency <n>",
    "number of concurrent operations",
    (value) => Number.parseInt(value),
    16
  )
  .option(
    "--prefix <s>",
    "filters to datasets/collections whose name starts with this prefix"
  )
  .option(
    "--age <duration>",
    "minimum age of datasets/collections to remove (e.g. 2d, 30m. default: 24h)",
    parseDuration("--age"),
    24 * 60 * 60 * 1000 // 24 hours
  )
  .option(
    "--timeout <duration>",
    "kill the command if it runs longer than this (e.g. 30m, 1h. default: 5m)",
    parseDuration("--timeout"),
    5 * 60 * 1000 // 5 minutes
  )
  .option("--dry-run", "do not perform the reset operation", false)
  .passThroughOptions()
  .allowUnknownOption()
  .action(async (cmd: string[], opts: Opts) => {
    if (cmd.length === 0) {
      program.help();
    }

    const env = loadEnv();
    const limit = pLimit(opts.concurrency);

    const client = createClient(env);

    // "Before" hook
    const beforeState = await discoverState(client, opts);
    if (!opts.dryRun) {
      await resetState(client, beforeState, limit);
    } else {
      log.warn(`Skipping reset.`);
    }

    // Run the command
    const exitCode = await exec(cmd, opts.dryRun, opts.timeout);

    // "After" hook
    const afterState = await discoverState(client, opts);
    if (!opts.dryRun) {
      await resetState(client, afterState, limit);
    } else {
      log.warn(`Skipping reset.`);
    }

    process.exit(exitCode);
  })
  .parseAsync();

async function discoverState(client: Client, opts: Opts): Promise<State> {
  log.info(`Discovering state for ${JSON.stringify(opts)}`);

  const [allDatasets, allCollections] = await Promise.all([
    withRetry(() => client.datasets().list()),
    withRetry(() => client.collections().list()),
  ]);
  log.info(
    `Found ${allDatasets.length} datasets and ${allCollections.length} collections`
  );

  const datasets = allDatasets.filter((d) => {
    if (opts.prefix && !d.name.startsWith(opts.prefix)) return false;
    if (opts.age && !olderThan(d.createdAt, opts.age)) return false;
    return true;
  });
  const collections = allCollections.filter((c) => {
    if (opts.prefix && !c.name.startsWith(opts.prefix)) return false;
    if (opts.age && !olderThan(c.createdAt, opts.age)) return false;
    return true;
  });
  log.info(
    `Filtered to ${datasets.length} datasets, ${collections.length} collections`
  );

  return { datasets, collections };
}

async function resetState(client: Client, state: State, limit: LimitFunction) {
  const futs: Promise<void>[] = [];

  for (const dataset of state.datasets) {
    futs.push(
      limit(async () => {
        try {
          await client.datasets().delete(dataset.name);
        } catch (error) {
          if (
            error instanceof Error &&
            error.message.includes("dataset not found")
          ) {
            return;
          }

          log.error(`Error deleting dataset ${dataset.name}:`, error);
        }
      })
    );
  }

  for (const collection of state.collections) {
    futs.push(
      limit(async () => {
        try {
          await client.collections().delete(collection.name);
        } catch (error) {
          if (
            error instanceof Error &&
            error.message.includes("collection not found")
          ) {
            return;
          }

          log.error(`Error deleting collection ${collection.name}:`, error);
        }
      })
    );
  }

  await Promise.all(futs);
}

// Client

function createClient(env: Env) {
  log.info(
    `Initializing client TOPK_REGION=${env.TOPK_REGION} TOPK_HOST=${env.TOPK_HOST} TOPK_HTTPS=${env.TOPK_HTTPS} TOPK_API_KEY=***`
  );
  return new Client({
    apiKey: env.TOPK_API_KEY,
    region: env.TOPK_REGION,
    host: env.TOPK_HOST,
    https: env.TOPK_HTTPS,
  });
}

function loadEnv() {
  const schema = z.object({
    TOPK_API_KEY: z.string(),
    TOPK_REGION: z.string(),
    TOPK_HOST: z.string(),
    TOPK_HTTPS: z.stringbool().default(true),
  });

  const result = schema.safeParse(process.env);
  if (!result.success) {
    log.error(`Invalid environment:\n${z.prettifyError(result.error)}`);
    process.exit(1);
  }
  return result.data;
}

// Command

async function exec(
  cmd: string[],
  dryRun: boolean,
  timeout: number
): Promise<number> {
  if (dryRun) {
    log.info(`Dry run: ${cmd.join(" ")}`);
    return 0;
  }

  const timeoutSignal = AbortSignal.timeout(timeout);
  let proc: ReturnType<typeof Bun.spawn>;
  try {
    // @ts-ignore
    proc = Bun.spawn({
      cmd,
      stdin: "inherit",
      stdout: "inherit",
      stderr: "inherit",
      signal: timeoutSignal,
    });
  } catch (error) {
    log.error(error instanceof Error ? error.message : String(error));
    return 127;
  }

  const signals: NodeJS.Signals[] = ["SIGINT", "SIGTERM", "SIGHUP", "SIGQUIT"];
  const forward = (sig: NodeJS.Signals) => () => {
    try {
      proc.kill(sig);
    } catch {}
  };
  const handlers = signals.map((sig) => {
    const h = forward(sig);
    process.on(sig, h);
    return [sig, h] as const;
  });

  try {
    const exitCode = await proc.exited;
    if (timeoutSignal.aborted) {
      log.error(`Command "${cmd.join(" ")}" timed out after ${timeout}ms`);
      return 124;
    }
    if (exitCode !== 0) {
      log.error(`Command "${cmd.join(" ")}" exited with code ${exitCode}`);
    }
    return exitCode;
  } finally {
    for (const [sig, h] of handlers) process.off(sig, h);
  }
}

// Utils

function withRetry<T>(fn: () => Promise<T>, opts?: RetryOptions): Promise<T> {
  return pRetry(fn, {
    retries: 5,
    minTimeout: 200,
    factor: 2,
    ...opts,
  });
}

function parseDuration(flag: string) {
  return (value: string): number => {
    const result = ms(value as StringValue);
    if (typeof result !== "number" || Number.isNaN(result)) {
      log.error(`Invalid ${flag} value: ${value} (expected e.g. 2d, 30m, 1h)`);
      process.exit(1);
    }
    return result;
  };
}

function olderThan(ts: string, age: number): boolean {
  const createdMs = Date.parse(ts);
  return Date.now() - createdMs >= age;
}
