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

export interface AppSettings {
	time_to_live: AppTtl;
	domain: string;
	registry: string;
	disallow_robots: boolean;
	app_blueprint: string;
	basic_auth: [string, string] | null;
	environment: Map<string, string> | null;
}

export interface App {
	name: string;
	status: string;
	services: AppService[];
	settings: AppSettings | null;
}
export interface TaskDetail {
	state: string;
	id: string;
	start_time: string;
	finish_time: string;
	stderr: string;
	stdout: string;
	app_name: null | string;
}

export interface ApiError {
	message: string;
	error: boolean;
}
