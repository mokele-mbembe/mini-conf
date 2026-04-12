export interface ProjectSummary {
  id: number;
  code: string;
  name: string;
  status: string;
  description: string | null;
}

export interface ProjectListResponse {
  items: ProjectSummary[];
}
