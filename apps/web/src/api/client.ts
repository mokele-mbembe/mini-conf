import { ApiRequestError } from "./error";
import type { ApiError } from "./error";

const CSRF_COOKIE_NAME = "mini_conf_csrf";
const CSRF_HEADER_NAME = "X-CSRF-Token";
const BASE_URL = normalizeBaseUrl(import.meta.env.VITE_API_BASE_URL ?? "/api");

let csrfToken: string | null = null;

function normalizeBaseUrl(value: string): string {
  const trimmed = value.trim();
  if (!trimmed || trimmed === "/") {
    return "";
  }

  return trimmed.replace(/\/+$/, "");
}

function buildUrl(path: string): string {
  const normalizedPath = path.startsWith("/") ? path : `/${path}`;
  return `${BASE_URL}${normalizedPath}`;
}

function isUnsafeMethod(method: string | undefined): boolean {
  return ["POST", "PUT", "PATCH", "DELETE"].includes(
    (method ?? "GET").toUpperCase(),
  );
}

function readCookie(name: string): string | null {
  if (typeof document === "undefined") {
    return null;
  }

  const prefix = `${name}=`;
  for (const part of document.cookie.split(";")) {
    const trimmed = part.trim();
    if (trimmed.startsWith(prefix)) {
      return trimmed.slice(prefix.length) || null;
    }
  }

  return null;
}

async function parseErrorResponse(res: Response): Promise<ApiRequestError> {
  try {
    const body = (await res.json()) as ApiError;
    return new ApiRequestError(res.status, body);
  } catch {
    return new ApiRequestError(res.status, {
      code: "unknown_error",
      message: res.statusText || "Request failed",
    });
  }
}

async function request<T>(path: string, options: RequestInit = {}): Promise<T> {
  const url = buildUrl(path);

  const headers: Record<string, string> = {
    Accept: "application/json",
    ...(options.headers as Record<string, string>),
  };

  if (options.body && typeof options.body === "string") {
    headers["Content-Type"] = "application/json";
  }

  if (isUnsafeMethod(options.method)) {
    const token = csrfToken ?? readCookie(CSRF_COOKIE_NAME);
    if (token) {
      headers[CSRF_HEADER_NAME] = token;
    }
  }

  let res: Response;
  try {
    res = await fetch(url, {
      ...options,
      headers,
      credentials: "include",
    });
  } catch {
    throw new ApiRequestError(0, {
      code: "network_error",
      message: "Network request failed",
    });
  }

  updateCsrfToken(res);

  if (res.status === 204) {
    return undefined as T;
  }

  if (!res.ok) {
    throw await parseErrorResponse(res);
  }

  return (await res.json()) as T;
}

function updateCsrfToken(res: Response): void {
  const token = res.headers.get(CSRF_HEADER_NAME)?.trim();
  if (token) {
    csrfToken = token;
  }
}

export const client = {
  get<T>(path: string): Promise<T> {
    return request<T>(path, { method: "GET" });
  },

  post<T>(path: string, body?: unknown): Promise<T> {
    return request<T>(path, {
      method: "POST",
      body: body !== undefined ? JSON.stringify(body) : undefined,
    });
  },

  put<T>(path: string, body?: unknown): Promise<T> {
    return request<T>(path, {
      method: "PUT",
      body: body !== undefined ? JSON.stringify(body) : undefined,
    });
  },

  patch<T>(path: string, body?: unknown): Promise<T> {
    return request<T>(path, {
      method: "PATCH",
      body: body !== undefined ? JSON.stringify(body) : undefined,
    });
  },

  delete<T>(path: string, body?: unknown): Promise<T> {
    return request<T>(path, {
      method: "DELETE",
      body: body !== undefined ? JSON.stringify(body) : undefined,
    });
  },
};
