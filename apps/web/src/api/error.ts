export interface ApiError {
  code: string;
  message: string;
  request_id?: string;
}

export class ApiRequestError extends Error {
  public readonly code: string;
  public readonly status: number;
  public readonly requestId?: string;

  constructor(status: number, error: ApiError) {
    super(error.message);
    this.name = "ApiRequestError";
    this.status = status;
    this.code = error.code;
    this.requestId = error.request_id;
  }
}

export function isApiError(err: unknown): err is ApiRequestError {
  return err instanceof ApiRequestError;
}
