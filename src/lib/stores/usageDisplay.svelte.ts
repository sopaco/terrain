import { t } from "../i18n";

export type UsageBadgePeriod = "day" | "month";

const STORAGE_KEY = "terrain.usage.badgePeriod";

function readStoredPeriod(): UsageBadgePeriod {
  if (typeof localStorage === "undefined") return "day";
  const raw = localStorage.getItem(STORAGE_KEY);
  return raw === "month" ? "month" : "day";
}

export const usageDisplay = $state({
  badgePeriod: readStoredPeriod() as UsageBadgePeriod,
});

export function setUsageBadgePeriod(period: UsageBadgePeriod) {
  usageDisplay.badgePeriod = period;
  if (typeof localStorage !== "undefined") {
    localStorage.setItem(STORAGE_KEY, period);
  }
}

export function usageBadgePeriodLabel(period: UsageBadgePeriod): string {
  return period === "month" ? t("usage.period.month") : t("usage.period.today");
}
