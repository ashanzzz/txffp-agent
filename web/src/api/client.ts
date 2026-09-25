export interface ApiResponse<T> {
  ok: boolean;
  data?: T;
  error?: {
    code: string;
    message: string;
    human_action?: {
      action_id: string;
      url: string;
    };
  };
}

export interface AuthState {
  status: string;
  message: string;
  session?: {
    session_id: string;
    profile?: {
      username?: string;
    };
    last_verified_at: string;
  };
  attempt_count: number;
  active_human_action_id?: string;
}

export interface SettingsData {
  credentials: {
    username_configured: boolean;
    password_configured: boolean;
    phone_configured: boolean;
    source: string;
  };
  auth_mode: string;
  auto_login: boolean;
  max_auth_attempts: number;
  steel_base_url: string;
  research_mode: boolean;
}

export interface EtcCard {
  card_id: string;
  card_no_masked: string;
  card_type: string;
  plate_number: string;
  status: string;
}

export interface TransactionRecord {
  record_id: string;
  card_id: string;
  plate_number: string;
  en_time: string;
  ex_time: string;
  en_station: string;
  ex_station: string;
  amount: string;
  invoice_status: string;
}

export interface InvoicePreview {
  start_date: string;
  end_date: string;
  total_records: number;
  invoiceable_records: number;
  total_amount: string;
  card_id?: string;
  title_name?: string;
  records: TransactionRecord[];
  can_submit: boolean;
}

export interface InvoiceItem {
  invoice_id: string;
  invoice_code?: string;
  invoice_number?: string;
  amount: string;
  issue_date: string;
  status: string;
  pdf_download_url?: string;
  summary_download_url?: string;
}

export const api = {
  async getHealth() {
    const res = await fetch('/api/v1/health');
    return (await res.json()) as ApiResponse<{
      status: string;
      service: string;
      version: string;
      steel_connected: boolean;
      database: string;
    }>;
  },

  async getAuthStatus() {
    const res = await fetch('/api/v1/auth/status');
    return (await res.json()) as ApiResponse<AuthState>;
  },

  async ensureAuth() {
    const res = await fetch('/api/v1/auth/ensure', { method: 'POST' });
    return (await res.json()) as ApiResponse<{
      status: string;
      message: string;
      session_active: boolean;
    }>;
  },

  async getCards() {
    const res = await fetch('/api/v1/cards');
    return (await res.json()) as ApiResponse<EtcCard[]>;
  },

  async previewInvoice(req: { start_date: string; end_date: string; card_id?: string }) {
    const res = await fetch('/api/v1/invoices/preview', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(req),
    });
    return (await res.json()) as ApiResponse<InvoicePreview>;
  },

  async createInvoice(req: { start_date: string; end_date: string; idempotency_key: string; card_id?: string }) {
    const res = await fetch('/api/v1/invoices', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(req),
    });
    return (await res.json()) as ApiResponse<{
      invoice_id: string;
      status: string;
      message: string;
      total_amount: string;
      record_count: number;
    }>;
  },

  async getInvoices() {
    const res = await fetch('/api/v1/invoices');
    return (await res.json()) as ApiResponse<InvoiceItem[]>;
  },

  async getSettings() {
    const res = await fetch('/api/v1/settings');
    return (await res.json()) as ApiResponse<SettingsData>;
  },

  async updateCredentials(creds: { username: string; password: string; phone?: string }) {
    const res = await fetch('/api/v1/settings/credentials', {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(creds),
    });
    return (await res.json()) as ApiResponse<{ updated: boolean }>;
  },

  async deleteCredentials() {
    const res = await fetch('/api/v1/settings/credentials', { method: 'DELETE' });
    return (await res.json()) as ApiResponse<{ deleted: boolean }>;
  },
};
