const ANSI_PATTERN = /\u001B\[[0-9;]*m/g;

export function stripAnsi(text) {
  return text.replace(ANSI_PATTERN, "");
}

export function createPalette(enabled) {
  const paint = code => text => enabled ? `\u001B[${code}m${text}\u001B[0m` : text;
  return {
    enabled,
    bold: paint("1"),
    dim: paint("2"),
    red: paint("31"),
    green: paint("32"),
    yellow: paint("33"),
    cyan: paint("36"),
    gray: paint("90"),
  };
}

const QUOTER_LABELS = {
  uniswap_v2: "v2",
  uniswap_v3: "v3",
  erc4626: "4626",
  chainlink: "cl",
  ecb: "ecb",
  fixed: "fixed",
};

export function formatRoute(sources = []) {
  if (sources.length === 0) return "—";
  return sources.map(identity => {
    const [kind, detail] = identity.split(":");
    const label = QUOTER_LABELS[kind] ?? kind;
    const address = detail?.startsWith("0x") ? detail.slice(2, 6) : undefined;
    return address === undefined ? label : `${label}:${address}`;
  }).join("→");
}

export function formatUsd(value) {
  if (value === undefined) return "—";
  if (value >= 1000) {
    const [integer, fraction] = value.toFixed(2).split(".");
    return `${integer.replace(/\B(?=(\d{3})+(?!\d))/g, ",")}.${fraction}`;
  }
  if (value >= 1) return value.toFixed(4);
  if (value >= 0.000_001) {
    return value.toPrecision(4).replace(/\.?0+$/, "");
  }
  return value.toExponential(2);
}

export function formatPercent(value, digits = 3) {
  if (value === undefined) return "—";
  if (value !== 0 && value < 10 ** -digits / 2) return `<${(10 ** -digits).toFixed(digits)}%`;
  return `${value.toFixed(digits)}%`;
}

export function formatDuration(milliseconds) {
  if (milliseconds === undefined) return "—";
  if (milliseconds >= 10_000) return `${(milliseconds / 1000).toFixed(1)}s`;
  if (milliseconds >= 1) return `${Math.round(milliseconds)}ms`;
  return `${(milliseconds * 1000).toFixed(0)}µs`;
}

export function formatNanoseconds(nanoseconds) {
  if (nanoseconds === undefined) return "—";
  if (nanoseconds >= 1_000_000) return `${(nanoseconds / 1_000_000).toFixed(2)}ms`;
  if (nanoseconds >= 1_000) return `${(nanoseconds / 1_000).toFixed(2)}µs`;
  return `${Math.round(nanoseconds)}ns`;
}

// "within": inside the reference envelope, so the deviation is smaller than the
// references' own disagreement. "near": outside, but by no more than the spread
// itself (or 0.25% when the references fully agree). "far": beyond that.
export function significanceOf(implementation, statistics) {
  if (implementation === undefined) return "missing";
  if (implementation.error !== undefined && implementation.priceUsd === undefined) return "error";
  const assessment = implementation.assessment;
  if (assessment === undefined) return "unreferenced";
  if (assessment.withinEnvelope) return "within";
  const tolerance = Math.max(statistics?.spreadPercent ?? 0, 0.25);
  return assessment.envelopeDistancePercent <= tolerance ? "near" : "far";
}

const SIGNIFICANCE_MARKS = {
  within: { mark: "●", color: "green" },
  near: { mark: "◐", color: "yellow" },
  far: { mark: "○", color: "red" },
  unreferenced: { mark: "·", color: "gray" },
  error: { mark: "✗", color: "red" },
  missing: { mark: " ", color: "gray" },
};

export function renderTable(columns, rows, palette) {
  const widths = columns.map((column, index) => Math.max(
    stripAnsi(column.label).length,
    ...rows.map(row => stripAnsi(row[index] ?? "").length),
  ));
  const pad = (text, index) => {
    const padding = " ".repeat(widths[index] - stripAnsi(text).length);
    return columns[index].align === "right" ? padding + text : text + padding;
  };
  const line = cells => `  ${cells.map((cell, index) => pad(cell ?? "", index)).join("  ")}`.trimEnd();
  return [
    line(columns.map(column => palette.bold(column.label))),
    `  ${widths.map(width => "─".repeat(width)).join("──")}`,
    ...rows.map(row => line(row)),
  ];
}

function significanceCell(implementation, statistics, palette) {
  const { mark, color } = SIGNIFICANCE_MARKS[significanceOf(implementation, statistics)];
  return palette[color](mark);
}

function section(title, palette) {
  return ["", palette.bold(palette.cyan(title))];
}

function renderHeader(report, palette) {
  const referenceNote = report.referencesFrom === undefined
    ? "live references"
    : `references from ${report.referencesFrom}`;
  return [
    palette.bold("eth-prices autorouter eval"),
    `  block ${palette.cyan(String(report.blockNumber))} · ${new URL(report.rpcUrl).host}` +
      ` · ${report.implementationsRun.join(" + ")} · ${referenceNote}` +
      ` · ${report.iterations} route iterations`,
  ];
}

function renderReferenceHealth(report, palette) {
  const bySource = new Map();
  for (const [asset, result] of Object.entries(report.references)) {
    for (const record of result.records) {
      const entry = bySource.get(record.source) ?? { ok: 0, failures: [] };
      entry.ok += 1;
      bySource.set(record.source, entry);
    }
    for (const failure of result.errors) {
      const entry = bySource.get(failure.source) ?? { ok: 0, failures: [] };
      entry.failures.push({ asset, error: failure.error });
      bySource.set(failure.source, entry);
    }
  }

  const lines = section("references", palette);
  for (const [source, entry] of [...bySource.entries()].sort(([left], [right]) => left.localeCompare(right))) {
    const ok = palette.green(`✓ ${entry.ok}`);
    if (entry.failures.length === 0) {
      lines.push(`  ${source.padEnd(14)} ${ok}`);
      continue;
    }
    const grouped = new Map();
    for (const failure of entry.failures) {
      grouped.set(failure.error, [...grouped.get(failure.error) ?? [], failure.asset]);
    }
    const details = [...grouped.entries()]
      .map(([error, assets]) => `${assets.join(", ")}: ${error}`)
      .join("; ");
    lines.push(`  ${source.padEnd(14)} ${ok} ${palette.red(`✗ ${entry.failures.length}`)} ${palette.dim(`(${details})`)}`);
  }
  return lines;
}

function performanceSummary(output) {
  if (output === undefined) return {};
  const quotes = output.runs.flatMap(run => run.quotes);
  const collect = key => quotes.map(quote => quote[key]).filter(value => value !== undefined).sort((a, b) => a - b);
  const middle = values => values.length === 0 ? undefined : values[Math.floor(values.length / 2)];
  return {
    discoveryMs: output.runs.reduce((total, run) => total + run.discoveryMs, 0),
    discoveredQuoters: output.runs.reduce((total, run) => total + run.discoveredQuoters, 0),
    medianQuoteMs: middle(collect("quoteMs")),
    medianRouteComputeNs: middle(collect("routeComputeNs")),
    rpcRequests: output.runs.reduce((total, run) => total + (run.totalRpcRequests ?? 0), 0) || undefined,
  };
}

function renderScores(report, palette) {
  const lines = section("scores", palette);
  const rows = report.implementationsRun.map(name => {
    const score = report.scores[name];
    const performance = performanceSummary(report.implementations[name]);
    return [
      name,
      score.priceScore === undefined ? "—" : score.priceScore.toFixed(2),
      formatPercent(score.meanAbsolutePercentageError),
      formatPercent(score.medianAbsolutePercentageError),
      `${score.pricedRoutes}/${score.expectedPricedRoutes}`,
      score.assessedCount === 0 ? "—" : `${score.withinEnvelopeCount}/${score.assessedCount}`,
      `${score.routeCoveragePercent.toFixed(0)}%`,
      formatDuration(performance.discoveryMs),
      String(performance.discoveredQuoters ?? "—"),
      formatDuration(performance.medianQuoteMs),
      formatNanoseconds(performance.medianRouteComputeNs),
      performance.rpcRequests === undefined ? "—" : String(performance.rpcRequests),
    ];
  });
  lines.push(...renderTable([
    { label: "impl" },
    { label: "score", align: "right" },
    { label: "mape", align: "right" },
    { label: "median", align: "right" },
    { label: "priced", align: "right" },
    { label: "in-band", align: "right" },
    { label: "cover", align: "right" },
    { label: "discovery", align: "right" },
    { label: "quoters", align: "right" },
    { label: "quote", align: "right" },
    { label: "compute", align: "right" },
    { label: "rpc", align: "right" },
  ], rows, palette));

  for (const name of report.implementationsRun) {
    const worst = report.scores[name].worstCase;
    if (worst !== undefined && worst.errorPercent >= 1) {
      lines.push(`  ${palette.yellow("worst")} ${name}: ${worst.case} at ${formatPercent(worst.errorPercent)}`);
    }
  }

  const parity = report.scores.parity;
  if (parity !== undefined && parity.comparedRoutes > 0) {
    const divergent = parity.divergentCases.length === 0
      ? palette.green("all routes agree exactly")
      : `max Δ ${formatPercent(parity.maxDifferencePercent)} (${parity.divergentCases[0].case})`;
    lines.push(`  parity: ${parity.exactMatches}/${parity.comparedRoutes} exact · ${divergent}`);
  }
  return lines;
}

function renderCases(report, palette) {
  const lines = section("cases", palette);
  const both = report.implementationsRun.length === 2;
  const columns = [
    { label: "case" },
    { label: "consensus", align: "right" },
    { label: "±spread", align: "right" },
    { label: "n", align: "right" },
  ];
  for (const name of report.implementationsRun) {
    columns.push({ label: name === "typescript" ? "ts" : name, align: "right" });
    columns.push({ label: "err", align: "right" });
    columns.push({ label: "", align: "left" });
  }
  if (both) columns.push({ label: "Δ r/t", align: "right" });
  columns.push({ label: "route" }, { label: "quote", align: "right" });

  const rows = report.comparisons.map(comparison => {
    const statistics = comparison.reference;
    const row = [
      comparison.asset,
      formatUsd(statistics?.median),
      statistics === undefined || statistics.count === 0 ? palette.gray("—") : formatPercent(statistics.spreadPercent, 2),
      String(statistics?.count ?? 0),
    ];
    for (const name of report.implementationsRun) {
      const implementation = comparison.implementations[name];
      const kind = significanceOf(implementation, statistics);
      row.push(kind === "error" ? palette.red("error") : formatUsd(implementation?.priceUsd));
      row.push(implementation?.assessment === undefined
        ? palette.gray("—")
        : formatPercent(implementation.assessment.errorPercent));
      row.push(significanceCell(implementation, statistics, palette));
    }
    if (both) {
      row.push(comparison.crossDifferencePercent === undefined
        ? palette.gray("—")
        : comparison.crossDifferencePercent === 0
          ? palette.green("0")
          : palette.yellow(formatPercent(comparison.crossDifferencePercent)));
    }
    const routes = report.implementationsRun
      .map(name => comparison.implementations[name]?.sources ?? [])
      .filter(sources => sources.length > 0);
    const routesDiffer = routes.length === 2 && routes[0].join() !== routes[1].join();
    row.push(routes.length === 0
      ? palette.gray("—")
      : formatRoute(routes[0]) + (routesDiffer ? palette.yellow(" ≠") : ""));
    const quoteMs = report.implementationsRun
      .map(name => comparison.implementations[name]?.quoteMs)
      .filter(value => value !== undefined);
    row.push(quoteMs.length === 0 ? palette.gray("—") : quoteMs.map(value => formatDuration(value)).join("/"));
    return row;
  });

  lines.push(...renderTable(columns, rows, palette));
  lines.push(`  ${palette.green("●")} within reference band · ${palette.yellow("◐")} outside by ≤ spread · ` +
    `${palette.red("○")} beyond · ${palette.gray("·")} no reference · ${palette.yellow("≠")} routes differ`);

  const divergentRoutes = report.comparisons.filter(comparison => {
    const routes = report.implementationsRun.map(name => comparison.implementations[name]?.sources ?? []);
    return routes.length === 2 && routes[0].length > 0 && routes[1].length > 0 && routes[0].join() !== routes[1].join();
  });
  for (const comparison of divergentRoutes) {
    lines.push(`  ${palette.yellow("≠")} ${comparison.asset}: rust ${formatRoute(comparison.implementations.rust.sources)}` +
      ` vs ts ${formatRoute(comparison.implementations.typescript.sources)}`);
  }
  return lines;
}

function renderErrors(report, palette) {
  const lines = [];

  for (const name of report.implementationsRun) {
    const output = report.implementations[name];
    for (const run of output?.runs ?? []) {
      if (run.error !== undefined) lines.push(`  ${palette.red("✗")} ${name} ${run.name}: discovery failed: ${run.error}`);
      for (const discoverer of run.discoveryReport?.discoverers ?? []) {
        if (discoverer.failures.length === 0) continue;
        const grouped = new Map();
        for (const failure of discoverer.failures) {
          const message = failure.message.replaceAll(/0x[0-9A-Fa-f]{40}/g, "<address>");
          grouped.set(message, [...grouped.get(message) ?? [], failure.target]);
        }
        for (const [message, targets] of grouped) {
          lines.push(`  ${palette.yellow("▲")} ${name} ${discoverer.identity}: ${targets.length} failure${targets.length === 1 ? "" : "s"}: ${message}`);
          const shown = targets.slice(0, 6);
          const more = targets.length - shown.length;
          lines.push(`      ${palette.dim(shown.join(", ") + (more > 0 ? ` … +${more} more` : ""))}`);
        }
      }
    }
    for (const comparison of report.comparisons) {
      const implementation = comparison.implementations[name];
      if (implementation?.error !== undefined) {
        lines.push(`  ${palette.red("✗")} ${name} ${comparison.asset}: ${implementation.error}`);
      }
    }
  }

  if (lines.length === 0) return [];
  return [...section("problems", palette), ...lines];
}

function renderBaseline(report, palette) {
  const baseline = report.baseline;
  if (baseline === undefined) return [];
  const lines = section(`vs baseline (${baseline.generatedAt} · block ${baseline.blockNumber})`, palette);

  const arrow = (delta, betterWhenLower) => {
    if (delta === undefined || Math.abs(delta) < 0.0005) return palette.gray("=");
    const improved = betterWhenLower ? delta < 0 : delta > 0;
    const symbol = `${delta > 0 ? "▲" : "▼"} ${Math.abs(delta).toFixed(3)}`;
    return improved ? palette.green(symbol) : palette.red(symbol);
  };

  for (const entry of baseline.scores) {
    lines.push(`  ${entry.implementation.padEnd(10)} ` +
      `score ${entry.priceScore.to?.toFixed(2) ?? "—"} ${arrow(delta(entry.priceScore), false)}` +
      ` · mape ${formatPercent(entry.meanAbsolutePercentageError.to)} ${arrow(delta(entry.meanAbsolutePercentageError), true)}` +
      ` · coverage ${entry.routeCoveragePercent.to?.toFixed(0) ?? "—"}% ${arrow(delta(entry.routeCoveragePercent), false)}`);
  }

  const movers = baseline.caseDeltas.slice(0, 8);
  for (const mover of movers) {
    const change = mover.to === undefined
      ? palette.red("lost route")
      : mover.from === undefined
        ? palette.green(`new route at ${formatPercent(mover.to)}`)
        : `${formatPercent(mover.from)} → ${formatPercent(mover.to)} ${arrow(mover.to - mover.from, true)}`;
    lines.push(`  ${mover.implementation} ${mover.case}: ${change}`);
  }
  if (baseline.caseDeltas.length > movers.length) {
    lines.push(palette.dim(`  … ${baseline.caseDeltas.length - movers.length} more case changes in the JSON report`));
  }
  return lines;
}

function delta(pair) {
  if (pair.from === undefined || pair.to === undefined) return undefined;
  return pair.to - pair.from;
}

export function renderReport(report, options = {}) {
  const palette = createPalette(options.colors ?? false);
  return [
    ...renderHeader(report, palette),
    ...renderReferenceHealth(report, palette),
    ...renderScores(report, palette),
    ...renderCases(report, palette),
    ...renderErrors(report, palette),
    ...renderBaseline(report, palette),
    "",
    palette.dim(`  report: ${report.reportPath}`),
  ].join("\n");
}
