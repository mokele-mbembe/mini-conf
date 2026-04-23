export interface SetupStatusResponse {
  setup_required: boolean;
  setup_completed_at: string | null;
  setup_completed_by_user_id: number | null;
  active_platform_admin_count: number;
  project_count: number;
}
