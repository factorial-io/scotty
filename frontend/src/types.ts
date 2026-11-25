import type { AppOperation } from './generated';

export interface AppService {
	service: string;
	domains: string[];
	url: string;
	status: string;
	started_at: string;
	use_tls: boolean;
	exit_code?: number | null;
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
	destroy_on_ttl: boolean;
	domain: string;
	registry: string;
	disallow_robots: boolean;
	app_blueprint: string;
	basic_auth: [string, string] | null;
	environment: Map<string, string> | null;
	middlewares: string[];
	scopes: string[];
}

export type LoadBalancerConnectivity =
	'Unknown' | 'NotApplicable' | 'Connected' | 'Disconnected' | 'LoadBalancerUnavailable';

export interface App {
	name: string;
	status: string;
	services: AppService[];
	settings: AppSettings | null;
	last_checked: string | null;
	/** When the server's next scheduled app-state sweep is due; null before the first sweep. */
	next_check: string | null;
	load_balancer_connectivity: LoadBalancerConnectivity;
	/** The operation running on the app; null when idle. */
	operation: AppOperation | null;
	/** When the TTL checker stops or destroys the app; null when forever or not running. */
	ttl_expires_at: string | null;
	/** Path prefixes the load balancer refuses for the app (global plus blueprint). */
	blocked_paths: string[];
}
// TaskDetail is now imported from generated types (TaskDetails)
// The old interface with stdout/stderr is deprecated
export type {
	TaskDetails as TaskDetail,
	WebSocketMessage,
	TaskOutputData,
	OutputLine,
	OutputStreamType,
	AppOperation
} from './generated';

export interface ApiError {
	message: string;
	error: boolean;
}

export interface BlueprintAction {
	description: string;
	permission?: string;
}

/** The flat JSON Schema subset a templated blueprint may declare for its parameters. */
export interface BlueprintParameterProperty {
	type: 'string' | 'number' | 'integer' | 'boolean';
	title?: string;
	description?: string;
	default?: string | number | boolean;
	enum?: (string | number)[];
	/** Set by the server for generated and derived parameters; forms skip these. */
	readOnly?: boolean;
}

export interface BlueprintParameterSchema {
	type?: 'object';
	properties?: Record<string, BlueprintParameterProperty>;
	required?: string[];
}

export interface BlueprintTemplate {
	parameters: BlueprintParameterSchema | null;
	images: string[];
	/** Glob patterns for caller files; empty means no upload. */
	payload: string[];
	/** Patterns each needing at least one matching file; omitted when empty. */
	payload_required?: string[];
	domain_suffix?: string | null;
}

export interface Blueprint {
	name: string;
	description: string;
	actions: Record<string, BlueprintAction>;
	/** True for directory blueprints, which can create apps; false for validation-only ones. */
	templated: boolean;
	template?: BlueprintTemplate;
	/** Path prefixes blocked for apps from this blueprint, in addition to the global ones. */
	blocked_paths: string[];
}

export interface BlueprintsResponse {
	blueprints: Record<string, Blueprint>;
	/** Path prefixes blocked for every app. */
	global_blocked_paths: string[];
}

export interface CustomAction {
	name: string;
	description: string;
	permission?: string;
}

/** One uploaded file; `content` is base64, of gzip data when `compressed`. */
export interface FileEntry {
	name: string;
	content: string;
	compressed: boolean;
}

/** `AppSettings` as the create endpoint takes it. */
export type CreateAppSettings = Omit<AppSettings, 'registry' | 'environment'> & {
	registry: string | null;
	environment: Record<string, string>;
	notify: unknown[];
};

/** Body of `POST apps/create`; the response is a `RunningAppContext` (task id in `task.id`). */
export interface CreateAppRequest {
	app_name: string;
	settings: CreateAppSettings;
	files: { files: FileEntry[] };
	custom_domains: { domain: string; service: string }[];
	requested_scopes: string[];
	blueprint_parameters?: Record<string, string | number | boolean>;
	image?: string;
}

export interface RunningAppContext {
	task: {
		id: string;
	};
	app_data?: App;
}

export interface OAuthConfig {
	enabled: boolean;
	provider: string;
	redirect_url: string;
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
