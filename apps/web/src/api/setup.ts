import { client } from "./client";
import type { SetupStatusResponse } from "./types/setup";

export function getSetupStatus(): Promise<SetupStatusResponse> {
  return client.get<SetupStatusResponse>("/setup/status");
}

export function completeSetup(): Promise<SetupStatusResponse> {
  return client.post<SetupStatusResponse>("/setup/complete");
}
