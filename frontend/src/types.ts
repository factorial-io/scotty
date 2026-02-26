export interface AppService {
	service: string;
	domains: string[];
	url: string;
	status: string;
	started_at: string;
	use_tls: boolean;
}

export enum AppTtlKeys {
	Hours = 'Hours',
	Days = 'Days'
}

/*
export type AppTtl = {
	[key in AppTtlKeys]: number | 'Forever';
};
*/
export type AppTtl =
	| {
			Hours?: number;
			Days?: number;
	  }
	| 'Forever';

export interface ServicePortMapping {
	service: string;
	port: number;
	domains: string[];
}

export interface AppSettings {
	public_services: ServicePortMapping[];
	time_to_live: AppTtl;
	domain: string;
	registry: string;
	disallow_robots: boolean;
	app_blueprint: string;
	basic_auth: [string, string] | null;
	environment: Map<string, string> | null;
	middlewares: string[];
}

export interface App {
	name: string;
	status: string;
	services: AppService[];
	settings: AppSettings | null;
	last_checked: string | null;
}
// TaskDetail is now imported from generated types (TaskDetails)
// The old interface with stdout/stderr is deprecated
export type {
	TaskDetails as TaskDetail,
	WebSocketMessage,
	TaskOutputData,
	OutputLine,
	OutputStreamType
} from './generated';

export interface ApiError {
	message: string;
	error: boolean;
}

export interface BlueprintAction {
	description: string;
}

export interface Blueprint {
	actions: Record<string, BlueprintAction>;
}

export interface BlueprintsResponse {
	blueprints: Record<string, Blueprint>;
}

export interface CustomAction {
	name: string;
	description: string;
}

export interface RunningAppContext {
	task?: {
		id: string;
	};
}

export interface OAuthConfig {
	enabled: boolean;
	provider: string;
	redirect_url: string;
	oauth2_proxy_base_url: string | null;
	oidc_issuer_url: string | null;
	client_id: string | null;
	device_flow_enabled: boolean;
}

export interface ServerInfo {
	domain: string;
	version: string;
	auth_mode: 'dev' | 'oauth' | 'bearer';
	oauth_config?: OAuthConfig;
}

export interface DeviceFlowResponse {
	device_code: string;
	user_code: string;
	verification_uri: string;
	expires_in: number;
	interval: number;
}

export interface TokenResponse {
	access_token: string;
	token_type: string;
	user_id: string;
	user_name: string;
	user_email: string;
	user_picture?: string;
}

export interface OAuthErrorResponse {
	error: string;
	error_description: string;
}

export interface UserInfo {
	id: string;
	name: string;
	email: string;
	picture?: string;
}
