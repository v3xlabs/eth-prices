export function median(values) {
  if (values.length === 0) return undefined;
  const sorted = [...values].sort((left, right) => left - right);
  const middle = Math.floor(sorted.length / 2);
  return sorted.length % 2 === 0 ? (sorted[middle - 1] + sorted[middle]) / 2 : sorted[middle];
}

export function percentageError(actual, expected) {
  return Math.abs(actual - expected) / expected * 100;
}

export function referenceStatistics(records = []) {
  const prices = records.map(record => record.priceUsd);
  if (prices.length === 0) return { count: 0, sources: [] };
  const consensus = median(prices);
  const min = Math.min(...prices);
  const max = Math.max(...prices);
  const medianAbsoluteDeviation = median(prices.map(price => Math.abs(price - consensus)));

  return {
    count: prices.length,
    min,
    max,
    mean: prices.reduce((total, price) => total + price, 0) / prices.length,
    median: consensus,
    spreadPercent: (max - min) / consensus * 100,
    medianAbsoluteDeviation,
    madPercent: medianAbsoluteDeviation / consensus * 100,
    sources: records.map(record => ({
      source: record.source,
      priceUsd: record.priceUsd,
      deviationPercent: (record.priceUsd - consensus) / consensus * 100,
      ...record.observedAt === undefined ? {} : { observedAt: record.observedAt },
    })),
  };
}

export function assessPrice(priceUsd, statistics) {
  if (priceUsd === undefined || statistics === undefined || statistics.count === 0) return undefined;
  const withinEnvelope = priceUsd >= statistics.min && priceUsd <= statistics.max;
  const envelopeDistance = priceUsd < statistics.min ? statistics.min - priceUsd : priceUsd - statistics.max;
  const nearest = statistics.sources.reduce((best, candidate) =>
    best === undefined || Math.abs(candidate.priceUsd - priceUsd) < Math.abs(best.priceUsd - priceUsd)
      ? candidate
      : best, undefined);

  return {
    errorPercent: percentageError(priceUsd, statistics.median),
    withinEnvelope,
    envelopeDistancePercent: withinEnvelope ? 0 : envelopeDistance / statistics.median * 100,
    deviationSigmas: statistics.count < 3 || statistics.medianAbsoluteDeviation === 0
      ? undefined
      : Math.abs(priceUsd - statistics.median) / statistics.medianAbsoluteDeviation,
    nearestSource: nearest?.source,
  };
}

export function implementationScore(comparisons, name) {
  const entries = comparisons.map(comparison => ({
    comparison,
    implementation: comparison.implementations[name],
  }));
  const referenced = entries.filter(({ comparison }) => comparison.reference !== undefined && comparison.reference.count > 0);
  const assessed = referenced.filter(({ implementation }) => implementation?.assessment !== undefined);
  const errors = assessed.map(({ implementation }) => implementation.assessment.errorPercent);
  const pricedRoutes = entries.filter(({ implementation }) => implementation?.priceUsd !== undefined).length;
  const pricePoints = referenced.reduce((total, { implementation }) => {
    const errorPercent = implementation?.assessment?.errorPercent;
    return total + (errorPercent === undefined ? 0 : Math.max(0, 100 - errorPercent));
  }, 0);
  const worst = assessed.reduce((current, entry) =>
    current === undefined || entry.implementation.assessment.errorPercent > current.implementation.assessment.errorPercent
      ? entry
      : current, undefined);

  return {
    priceScore: referenced.length === 0 ? undefined : pricePoints / referenced.length,
    meanAbsolutePercentageError: errors.length === 0
      ? undefined
      : errors.reduce((total, error) => total + error, 0) / errors.length,
    medianAbsolutePercentageError: median(errors),
    withinEnvelopeCount: assessed.filter(({ implementation }) => implementation.assessment.withinEnvelope).length,
    assessedCount: assessed.length,
    pricedRoutes: errors.length,
    expectedPricedRoutes: referenced.length,
    routeCoveragePercent: comparisons.length === 0 ? 0 : pricedRoutes / comparisons.length * 100,
    worstCase: worst === undefined
      ? undefined
      : { case: worst.comparison.case, errorPercent: worst.implementation.assessment.errorPercent },
  };
}

export function parityScore(comparisons) {
  const compared = comparisons
    .map(comparison => ({
      case: comparison.case,
      rustUsd: comparison.implementations.rust?.priceUsd,
      typescriptUsd: comparison.implementations.typescript?.priceUsd,
    }))
    .filter(entry => entry.rustUsd !== undefined && entry.typescriptUsd !== undefined)
    .map(entry => ({ case: entry.case, differencePercent: percentageError(entry.rustUsd, entry.typescriptUsd) }));
  const differences = compared.map(entry => entry.differencePercent);

  return {
    exactMatches: compared.filter(entry => entry.differencePercent === 0).length,
    comparedRoutes: compared.length,
    maxDifferencePercent: differences.length === 0 ? undefined : Math.max(...differences),
    meanDifferencePercent: differences.length === 0
      ? undefined
      : differences.reduce((total, difference) => total + difference, 0) / differences.length,
    divergentCases: compared
      .filter(entry => entry.differencePercent > 0)
      .sort((left, right) => right.differencePercent - left.differencePercent),
  };
}
