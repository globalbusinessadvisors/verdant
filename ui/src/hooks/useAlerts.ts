import { useCallback, useEffect, useState } from "react";
import type { AlertPort } from "../lib/ports";
import type { Alert, AlertRule } from "../lib/types";

export interface UseAlertsResult {
  alerts: Alert[];
  isLoading: boolean;
  configureRule: (rule: AlertRule) => Promise<void>;
  dismissAlert: (alertId: string) => Promise<void>;
  refresh: () => Promise<void>;
}

export function useAlerts(alertPort: AlertPort): UseAlertsResult {
  const [alerts, setAlerts] = useState<Alert[]>([]);
  const [isLoading, setIsLoading] = useState(true);

  const refresh = useCallback(async () => {
    setIsLoading(true);
    try {
      setAlerts(await alertPort.fetchAlerts());
    } finally {
      setIsLoading(false);
    }
  }, [alertPort]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const configureRule = useCallback(
    async (rule: AlertRule) => {
      await alertPort.configureRule(rule);
      await refresh();
    },
    [alertPort, refresh],
  );

  const dismissAlert = useCallback(
    async (alertId: string) => {
      await alertPort.dismissAlert(alertId);
      await refresh();
    },
    [alertPort, refresh],
  );

  return { alerts, isLoading, configureRule, dismissAlert, refresh };
}
