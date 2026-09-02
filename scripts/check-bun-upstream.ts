/**
 * Check the pinned Bun and WebKit baselines against upstream metadata.
 *
 * This intentionally reports updates instead of changing source pins. The
 * embedded Bun tree is a customized source tree, so every update needs the
 * TSP-specific native and worker verification described in the documentation.
 */

type JsonObject = Record<string, unknown>;

type Baseline = {
  bun: {
    repository: string;
    version: string;
    revision: string;
    releaseTag: string | null;
  };
  bootstrap: {
    repository: string;
    version: string;
  };
  webkit: {
    repository: string;
    revision: string;
    releaseTag: string | null;
  };
  security: {
    knownAdvisoryIds: string[];
  };
};

type Release = {
  tag_name: string;
  name: string;
  html_url: string;
  published_at: string | null;
  prerelease: boolean;
  draft: boolean;
  body: string | null;
};

type Advisory = {
  ghsa_id: string;
  cve_id: string | null;
  summary: string;
  severity: string | null;
  html_url: string;
  published_at: string | null;
  updated_at: string | null;
  vulnerabilities?: Array<{ package?: { ecosystem?: string; name?: string } }>;
};

const baselineUrl = new URL("../docs/reference/dependencies/bun-upstream.json", import.meta.url);
const bunPackageUrl = new URL("../bun/package.json", import.meta.url);
const webkitSourceUrl = new URL("../bun/scripts/build/deps/webkit.ts", import.meta.url);
const githubApi = "https://api.github.com";
const bunRepository = "oven-sh/bun";

const token = process.env.GITHUB_TOKEN;
const repository = process.env.GITHUB_REPOSITORY;
const isCi = process.argv.includes("--ci");
const isJson = process.argv.includes("--json");

function asObject(value: unknown, name: string): JsonObject {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error(`${name} must be an object`);
  }
  return value as JsonObject;
}

function asString(value: unknown, name: string): string {
  if (typeof value !== "string" || value.length === 0) throw new Error(`${name} must be a non-empty string`);
  return value;
}

function asStringArray(value: unknown, name: string): string[] {
  if (!Array.isArray(value) || value.some(item => typeof item !== "string")) {
    throw new Error(`${name} must be an array of strings`);
  }
  return value as string[];
}

function readBaseline(value: unknown): Baseline {
  const object = asObject(value, "baseline");
  const bun = asObject(object.bun, "baseline.bun");
  const bootstrap = asObject(object.bootstrap, "baseline.bootstrap");
  const webkit = asObject(object.webkit, "baseline.webkit");
  const security = asObject(object.security, "baseline.security");

  return {
    bun: {
      repository: asString(bun.repository, "baseline.bun.repository"),
      version: asString(bun.version, "baseline.bun.version"),
      revision: asString(bun.revision, "baseline.bun.revision"),
      releaseTag: bun.releaseTag === null ? null : asString(bun.releaseTag, "baseline.bun.releaseTag"),
    },
    bootstrap: {
      repository: asString(bootstrap.repository, "baseline.bootstrap.repository"),
      version: asString(bootstrap.version, "baseline.bootstrap.version"),
    },
    webkit: {
      repository: asString(webkit.repository, "baseline.webkit.repository"),
      revision: asString(webkit.revision, "baseline.webkit.revision"),
      releaseTag: webkit.releaseTag === null ? null : asString(webkit.releaseTag, "baseline.webkit.releaseTag"),
    },
    security: {
      knownAdvisoryIds: asStringArray(security.knownAdvisoryIds, "baseline.security.knownAdvisoryIds"),
    },
  };
}

async function github<T>(path: string, init: RequestInit = {}): Promise<{ response: Response; value: T | null }> {
  const headers = new Headers(init.headers);
  headers.set("Accept", "application/vnd.github+json");
  headers.set("User-Agent", "risol-tsp-bun-upstream-check");
  headers.set("X-GitHub-Api-Version", "2022-11-28");
  if (token) headers.set("Authorization", `Bearer ${token}`);

  const response = await fetch(`${githubApi}${path}`, { ...init, headers });
  const text = await response.text();
  let value: T | null = null;
  if (text.length > 0) {
    try {
      value = JSON.parse(text) as T;
    } catch {
      throw new Error(`GitHub returned invalid JSON for ${path}`);
    }
  }
  return { response, value };
}

function parseVersion(value: string): number[] | null {
  const match = value.match(/(?:^|v)(\d+)\.(\d+)\.(\d+)/);
  return match ? match.slice(1).map(Number) : null;
}

function compareVersions(left: string, right: string): number {
  const a = parseVersion(left);
  const b = parseVersion(right);
  if (!a || !b) return left.localeCompare(right);
  for (let index = 0; index < 3; index++) {
    if (a[index] !== b[index]) return a[index] - b[index];
  }
  return 0;
}

function releaseIsNewer(latest: Release, baseline: Baseline): boolean {
  if (baseline.bun.releaseTag === latest.tag_name) return false;
  return compareVersions(latest.tag_name, baseline.bun.version) > 0;
}

function advisorySummary(advisories: Advisory[]): string {
  return advisories
    .map(advisory => {
      const cve = advisory.cve_id ? ` (${advisory.cve_id})` : "";
      const severity = advisory.severity ? `, ${advisory.severity}` : "";
      return `- [${advisory.ghsa_id}](${advisory.html_url})${cve}${severity}: ${advisory.summary}`;
    })
    .join("\n");
}

function releaseSummary(release: Release): string {
  const body = (release.body ?? "").replace(/\r/g, "").trim();
  const securityLines = body
    .split("\n")
    .filter(line => /security|cve-|ghsa-|vulnerab|xss|sandbox|permission/i.test(line))
    .slice(0, 12);
  const details = securityLines.length > 0 ? `\n\n可能与安全相关的 release note 行：\n${securityLines.join("\n")}` : "";
  return `- [${release.name || release.tag_name}](${release.html_url})${release.published_at ? `，发布时间 ${release.published_at}` : ""}${details}`;
}

async function readLocalPins(): Promise<{ sourceVersion: string; webkitRevision: string }> {
  const packageJson = asObject(JSON.parse(await Bun.file(bunPackageUrl).text()), "bun/package.json");
  const webkitSource = await Bun.file(webkitSourceUrl).text();
  const webkitMatch = webkitSource.match(/export const WEBKIT_VERSION = "([0-9a-f]{40})"/);
  if (!webkitMatch) throw new Error("Could not find WEBKIT_VERSION in bun/scripts/build/deps/webkit.ts");
  return {
    sourceVersion: asString(packageJson.version, "bun/package.json.version"),
    webkitRevision: webkitMatch[1],
  };
}

async function loadAdvisories(baseline: Baseline): Promise<{ advisories: Advisory[]; unavailable: string | null }> {
  const result = await github<Advisory[]>(`/repos/${bunRepository}/security-advisories?per_page=100`);
  if (result.response.ok && Array.isArray(result.value)) return { advisories: result.value, unavailable: null };
  if (result.response.status === 404 || result.response.status === 403) {
    return {
      advisories: [],
      unavailable: `Bun repository advisories API returned HTTP ${result.response.status}; release monitoring remains active.`,
    };
  }
  throw new Error(`Could not read Bun security advisories: HTTP ${result.response.status}`);
}

async function createIssue(title: string, body: string): Promise<string | null> {
  if (!isCi || !token || !repository) return null;

  const search = await github<JsonObject>(`/search/issues?q=${encodeURIComponent(`repo:${repository} is:issue in:title ${title}`)}`);
  const items = Array.isArray(search.value?.items) ? search.value.items : [];
  if (items.length > 0) return String((items[0] as JsonObject).html_url ?? "existing issue");

  const created = await github<JsonObject>(`/repos/${repository}/issues`, {
    method: "POST",
    body: JSON.stringify({ title, body }),
    headers: { "Content-Type": "application/json" },
  });
  if (!created.response.ok) throw new Error(`Could not create tracking issue: HTTP ${created.response.status}`);
  return String(created.value?.html_url ?? "created issue");
}

async function writeSummary(text: string): Promise<void> {
  const summaryPath = process.env.GITHUB_STEP_SUMMARY;
  if (!summaryPath) return;
  await Bun.write(summaryPath, text + "\n");
}

const baseline = readBaseline(await (await fetch(baselineUrl)).json());
const localPins = await readLocalPins();
const latestReleaseResult = await github<Release>(`/repos/${bunRepository}/releases/latest`);
if (!latestReleaseResult.response.ok || !latestReleaseResult.value) {
  throw new Error(`Could not read latest Bun release: HTTP ${latestReleaseResult.response.status}`);
}

const latestRelease = latestReleaseResult.value;
const releaseUpdate = releaseIsNewer(latestRelease, baseline);
const latestWebkitResult = await github<Release>(`/repos/oven-sh/WebKit/releases/latest`);
if (!latestWebkitResult.response.ok || !latestWebkitResult.value) {
  throw new Error(`Could not read latest WebKit release: HTTP ${latestWebkitResult.response.status}`);
}
const latestWebkitRelease = latestWebkitResult.value;
const expectedWebkitTag = baseline.webkit.releaseTag ?? `autobuild-${baseline.webkit.revision}`;
const webkitReleaseUpdate = latestWebkitRelease.tag_name !== expectedWebkitTag;
const advisoryResult = await loadAdvisories(baseline);
const knownAdvisories = new Set(baseline.security.knownAdvisoryIds);
const newAdvisories = advisoryResult.advisories.filter(advisory => !knownAdvisories.has(advisory.ghsa_id));
const securityNote = advisoryResult.unavailable;
const findings: string[] = [];

if (localPins.sourceVersion !== baseline.bun.version) {
  findings.push(
    `## Baseline drift\n\nThe embedded Bun source reports version ${localPins.sourceVersion}, but the tracking baseline records ${baseline.bun.version}. Update the baseline metadata after confirming the source revision.`,
  );
}

if (localPins.webkitRevision !== baseline.webkit.revision) {
  findings.push(
    `## WebKit baseline drift\n\nThe build source pins WebKit ${localPins.webkitRevision}, but the tracking baseline records ${baseline.webkit.revision}. Review the engine change before updating metadata.`,
  );
}

if (webkitReleaseUpdate) {
  findings.push(
    `## WebKit release update\n\n当前 TSP WebKit baseline 为 ${baseline.webkit.revision}，上游最新 autobuild 为 [${latestWebkitRelease.tag_name}](${latestWebkitRelease.html_url})。请先确认 ABI、JSC 修复和 TSP embedded-worker 回归测试，再决定是否升级。`,
  );
}

if (releaseUpdate) {
  findings.push(`## Bun release update\n\n当前 TSP Bun baseline 为 ${baseline.bun.version} (${baseline.bun.revision})，上游发现更新：\n\n${releaseSummary(latestRelease)}`);
}

if (newAdvisories.length > 0) {
  findings.push(`## Bun security advisories\n\n检测到尚未记录的上游 advisory：\n\n${advisorySummary(newAdvisories)}`);
}

const report = {
  checkedAt: new Date().toISOString(),
  baseline,
  latestRelease: {
    tag: latestRelease.tag_name,
    url: latestRelease.html_url,
    publishedAt: latestRelease.published_at,
  },
  latestWebkitRelease: {
    tag: latestWebkitRelease.tag_name,
    url: latestWebkitRelease.html_url,
    publishedAt: latestWebkitRelease.published_at,
  },
  localPins,
  releaseUpdate,
  webkitReleaseUpdate,
  newAdvisories: newAdvisories.map(advisory => advisory.ghsa_id),
  advisoryApiNote: securityNote,
  findings,
};

if (isJson) {
  console.log(JSON.stringify(report, null, 2));
} else {
  console.log(`Bun upstream check: ${findings.length > 0 ? `${findings.length} finding(s)` : "no actionable updates"}`);
  console.log(`Current baseline: ${baseline.bun.version} (${baseline.bun.revision})`);
  console.log(`Latest release: ${latestRelease.tag_name}`);
  console.log(`Latest WebKit: ${latestWebkitRelease.tag_name}`);
  for (const finding of findings) console.log(`\n${finding}`);
  if (securityNote) console.log(`Note: ${securityNote}`);
}

if (findings.length > 0) {
  const titleParts = [];
  if (releaseUpdate) titleParts.push(`Bun ${latestRelease.tag_name}`);
  if (webkitReleaseUpdate) titleParts.push(`WebKit ${latestWebkitRelease.tag_name}`);
  if (newAdvisories.length > 0) titleParts.push(`${newAdvisories.length} security advisory`);
  const title = `[upstream] Review ${titleParts.join(" + ")}`;
  const body = [
    "This issue was created automatically by the Bun upstream tracker.",
    "",
    ...findings,
    ...(securityNote ? [`## Advisory API note\n\n${securityNote}`] : []),
    "",
    "### Required TSP verification",
    "",
    "- Review Bun release notes and the upstream fix commits.",
    "- Review the pinned WebKit/JSC revision when the native engine is involved.",
    "- Run the focused Rust tests, Linux embedded-worker release build, Windows embedded-worker smoke test, and TSP smoke test.",
    "- Update `docs/reference/dependencies/bun-upstream.json` only after the tested baseline is accepted.",
  ].join("\n");
  const issue = await createIssue(title, body);
  if (issue) console.log(`Tracking issue: ${issue}`);
  await writeSummary(`${findings.join("\n\n")}\n\n${issue ? `Tracking issue: ${issue}` : "Local check: no issue created."}`);
} else {
  await writeSummary(
    `## Bun upstream check\n\nNo actionable Bun or WebKit release or security advisory updates were found.\n\n- Baseline: ${baseline.bun.version} (${baseline.bun.revision})\n- Latest release: ${latestRelease.tag_name}\n- WebKit baseline: ${baseline.webkit.revision}\n- Latest WebKit: ${latestWebkitRelease.tag_name}${securityNote ? `\n\n> Note: ${securityNote}` : ""}`,
  );
}
