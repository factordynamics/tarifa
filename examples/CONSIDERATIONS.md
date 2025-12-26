# Backtest Considerations & Future Improvements

This document outlines the known limitations of the current backtest examples and potential improvements to make them more realistic.

## Current Limitations

### 1. Look-Ahead Bias (Critical)

**Problem:** The quality, value, and multifactor strategies fetch the *most recent* annual fundamentals (ratios, margins, earnings yield) and apply them retroactively to the entire backtest period.

**Impact:** Significantly inflates returns. At January 2023, we wouldn't know META's 2024 net margin.

**Solution:**
- Implement point-in-time (PIT) fundamental data
- Fetch historical fundamentals with filing dates
- Only use data that was actually available at each backtest date
- Account for reporting lag (10-K filings come 60-90 days after fiscal year end)

### 2. Survivorship Bias

**Problem:** The universe is 10 hand-picked mega-cap winners (AAPL, NVDA, META, etc.). No failed, delisted, or underperforming stocks are included.

**Impact:** Overstates returns since we're only backtesting on stocks that we know survived.

**Solution:**
- Use historical index constituents (e.g., S&P 500 as of each date)
- Include delisted stocks with proper handling of delistings
- Random or systematic universe selection (e.g., top N by market cap at each rebalance)

### 3. Period Selection Bias

**Problem:** 2023-2024 was a massive tech bull run driven by AI hype. Momentum strategies on tech stocks naturally excel here.

**Impact:** Results may not generalize to other market regimes.

**Solution:**
- Backtest across multiple market cycles (bull, bear, sideways)
- Include stress periods (2008, 2020, 2022)
- Report performance by regime
- Use longer backtest periods (10+ years)

### 4. Transaction Costs

**Problem:** Zero slippage, commissions, or market impact modeled.

**Impact:** Overstates net returns, especially for frequently rebalanced strategies.

**Solution:**
- Model commission costs (e.g., $0.005/share)
- Estimate slippage based on bid-ask spreads
- Model market impact for larger positions
- Consider turnover in strategy evaluation

### 5. Unrealistic Short Selling

**Problem:** Assumes perfect short availability with no borrow costs or constraints.

**Impact:** Short positions may be impossible or expensive in practice.

**Solution:**
- Model borrow costs (varies by stock, typically 0.5-5% annually)
- Check short availability / hard-to-borrow lists
- Consider long-only variants of strategies
- Model short squeeze risk

### 6. No Risk Management

**Problem:** Strategies run with fixed position sizes regardless of volatility or drawdowns.

**Impact:** May take excessive risk during volatile periods.

**Solution:**
- Implement position sizing based on volatility
- Add stop-loss or drawdown limits
- Consider volatility targeting
- Risk parity weighting across factors

### 7. Rebalance Timing

**Problem:** Rebalancing happens at arbitrary intervals (monthly/quarterly) on any trading day.

**Impact:** May miss optimal rebalancing points or incur unnecessary turnover.

**Solution:**
- Rebalance on specific dates (month-end, quarter-end)
- Threshold-based rebalancing (only when positions drift significantly)
- Consider rebalancing costs in timing decisions

### 8. Single Point Estimates

**Problem:** Reports single return/Sharpe numbers without confidence intervals.

**Impact:** Can't assess statistical significance or reliability of results.

**Solution:**
- Bootstrap confidence intervals
- Monte Carlo simulation
- Walk-forward validation
- Out-of-sample testing

## Implementation Roadmap

### Phase 1: Data Infrastructure
- [ ] Historical fundamental data with filing dates
- [ ] Historical index constituents
- [ ] Delisted stock data

### Phase 2: Realistic Execution
- [ ] Transaction cost model
- [ ] Short borrow cost model
- [ ] Slippage estimation

### Phase 3: Robust Backtesting
- [ ] Point-in-time data alignment
- [ ] Longer backtest periods
- [ ] Multiple market regimes

### Phase 4: Statistical Rigor
- [ ] Confidence intervals
- [ ] Walk-forward validation
- [ ] Out-of-sample testing
- [ ] Multiple testing corrections

## References

- Bailey, D. H., & Lopez de Prado, M. (2014). The Deflated Sharpe Ratio
- Harvey, C. R., Liu, Y., & Zhu, H. (2016). ... and the Cross-Section of Expected Returns
- Arnott, R. D., Harvey, C. R., & Markowitz, H. (2019). A Backtesting Protocol in the Era of Machine Learning
