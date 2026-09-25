//! How the tax tables are carried past the last known year: every indexed
//! dollar value scaled by the inflation between that year and the one asked
//! for.

use crate::plan::Dollars;

use super::{Bracket, ContributionLimits, PerStatus, PhaseOut, TaxParams};

/// How prices moved: each year's inflation over a span of years, and one
/// rate for every year before or after it.
#[derive(Debug, Clone, PartialEq)]
pub struct Inflation {
    /// The year `rates[0]` carries prices into, from the year before.
    first_year: i16,
    rates: Vec<f64>,
    beyond: f64,
}

impl Inflation {
    /// The same rate every year.
    #[must_use]
    pub fn constant(rate: f64) -> Self {
        Self {
            first_year: 0,
            rates: Vec::new(),
            beyond: rate,
        }
    }

    /// `rates[i]` carries prices from `first_year + i - 1` into
    /// `first_year + i`; `beyond` every year outside them.
    #[must_use]
    pub fn yearly(first_year: i16, rates: Vec<f64>, beyond: f64) -> Self {
        Self {
            first_year,
            rates,
            beyond,
        }
    }

    fn rate_into(&self, year: i16) -> f64 {
        usize::try_from(year - self.first_year)
            .ok()
            .and_then(|at| self.rates.get(at))
            .copied()
            .unwrap_or(self.beyond)
    }

    /// What a price in `from` costs in `to`; below one when `to` is
    /// earlier. Each run of years at one rate compounds as a single power,
    /// so a constant rate gives exactly `(1 + rate)^(to - from)`.
    #[must_use]
    pub fn factor(&self, from: i16, to: i16) -> f64 {
        let (low, high, sign) = if to >= from {
            (from, to, 1)
        } else {
            (to, from, -1)
        };
        let compound =
            |factor: f64, rate: f64, years: i32| factor * (1.0 + rate).powi(sign * years);
        let (factor, rate, years) =
            (low + 1..=high).fold((1.0, 0.0_f64, 0), |(factor, run_rate, run_years), year| {
                let rate = self.rate_into(year);
                if rate.to_bits() == run_rate.to_bits() {
                    (factor, run_rate, run_years + 1)
                } else {
                    (compound(factor, run_rate, run_years), rate, 1)
                }
            });
        compound(factor, rate, years)
    }
}

pub(crate) fn scale(amount: Dollars, factor: f64) -> Dollars {
    (amount as f64 * factor).round() as Dollars
}

fn scale_phase_out(bands: PerStatus<PhaseOut>, factor: f64) -> PerStatus<PhaseOut> {
    let scaled = |band: PhaseOut| PhaseOut {
        from: scale(band.from, factor),
        to: scale(band.to, factor),
    };
    PerStatus {
        single: scaled(bands.single),
        married_joint: scaled(bands.married_joint),
    }
}

fn scale_status(values: PerStatus<Dollars>, factor: f64) -> PerStatus<Dollars> {
    PerStatus {
        single: scale(values.single, factor),
        married_joint: scale(values.married_joint, factor),
    }
}

fn scale_brackets(brackets: &mut PerStatus<Vec<Bracket>>, factor: f64) {
    for bracket in brackets
        .single
        .iter_mut()
        .chain(&mut brackets.married_joint)
    {
        if !bracket.unindexed {
            bracket.over = scale(bracket.over, factor);
        }
    }
}

pub(super) fn inflate(base: &TaxParams, year: i16, factor: f64) -> TaxParams {
    let mut params = base.clone();
    params.year = year;
    params.deductions.standard = scale_status(base.deductions.standard, factor);
    scale_brackets(&mut params.brackets, factor);
    for state in params.states.values_mut() {
        state.deduction = scale_status(state.deduction, factor);
        scale_brackets(&mut state.brackets, factor);
    }
    params.ltcg.zero_until = scale_status(base.ltcg.zero_until, factor);
    params.ltcg.fifteen_until = scale_status(base.ltcg.fifteen_until, factor);
    params.limits = ContributionLimits {
        employer_plan: scale(base.limits.employer_plan, factor),
        employer_plan_catch_up_50: scale(base.limits.employer_plan_catch_up_50, factor),
        employer_plan_catch_up_60: scale(base.limits.employer_plan_catch_up_60, factor),
        overall_plan: scale(base.limits.overall_plan, factor),
        simple: scale(base.limits.simple, factor),
        simple_catch_up_50: scale(base.limits.simple_catch_up_50, factor),
        simple_catch_up_60: scale(base.limits.simple_catch_up_60, factor),
        ira: scale(base.limits.ira, factor),
        ira_catch_up_50: scale(base.limits.ira_catch_up_50, factor),
        hsa_self: scale(base.limits.hsa_self, factor),
        hsa_family: scale(base.limits.hsa_family, factor),
        hsa_catch_up_55: scale(base.limits.hsa_catch_up_55, factor),
        roth_ira_phase_out: scale_phase_out(base.limits.roth_ira_phase_out, factor),
        ira_deduction_phase_out: scale_phase_out(base.limits.ira_deduction_phase_out, factor),
    };
    for tier in &mut params.irmaa {
        tier.magi_over = scale_status(tier.magi_over, factor);
        tier.part_b = scale(tier.part_b, factor);
        tier.part_d = scale(tier.part_d, factor);
    }
    params
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_constant_rate_compounds_as_one_power_either_way() {
        let inflation = Inflation::constant(0.025);
        let bits = |from, to| inflation.factor(from, to).to_bits();
        assert_eq!(bits(2026, 2040), 1.025_f64.powi(14).to_bits());
        assert_eq!(bits(2040, 2026), 1.025_f64.powi(-14).to_bits());
        assert_eq!(bits(2030, 2030), 1.0_f64.to_bits());
    }

    #[test]
    fn yearly_rates_apply_inside_their_span_and_the_rate_beyond_outside() {
        let inflation = Inflation::yearly(2027, vec![0.10, 0.20], 0.0);
        assert!((inflation.factor(2026, 2028) - 1.1 * 1.2).abs() < 1e-12);
        assert!((inflation.factor(2020, 2035) - 1.1 * 1.2).abs() < 1e-12);
        assert!((inflation.factor(2028, 2026) - 1.0 / (1.1 * 1.2)).abs() < 1e-12);
    }
}
