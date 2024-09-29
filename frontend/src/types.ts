export interface AppService {
	service: string;
	domain: string;
	url: string;
	status: string;
	started_at: string;
}
export interface AppSettings {
	time_to_live: unknown;
	domain: string;
	registry: string;
	disallow_robots: boolean;
	blueprint: string;
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
