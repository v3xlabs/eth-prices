export function implementationScore(comparisons, usdKey, errorKey) {
  const priced = comparisons.filter(comparison => comparison.consensusUsd !== undefined);
  const successfulErrors = priced
    .map(comparison => comparison[errorKey])
    .filter(error => error !== undefined);
  const routeCount = comparisons.filter(comparison => comparison[usdKey] !== undefined).length;
  const pricePoints = priced.reduce((total, comparison) => {
    const error = comparison[errorKey];
    return total + (error === undefined ? 0 : Math.max(0, 100 - error));
  }, 0);

  return {
    priceScore: priced.length === 0 ? undefined : pricePoints / priced.length,
    meanAbsolutePercentageError: successfulErrors.length === 0
      ? undefined
      : successfulErrors.reduce((total, error) => total + error, 0) / successfulErrors.length,
    pricedRoutes: successfulErrors.length,
    expectedPricedRoutes: priced.length,
    routeCoveragePercent: comparisons.length === 0 ? 0 : routeCount / comparisons.length * 100,
  };
}

export function parityScore(comparisons) {
  const compared = comparisons.filter(comparison =>
    comparison.rustUsd !== undefined && comparison.typescriptUsd !== undefined
  );
  return {
    exactMatches: compared.filter(comparison => comparison.crossLanguageDifferencePercent === 0).length,
    comparedRoutes: compared.length,
  };
}
