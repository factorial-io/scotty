// See https://kit.svelte.dev/docs/types#app
// for information about these interfaces
declare global {
	interface AppService {
		service: string;
		domain: string;
	}
	interface App {
		name: string;
		status: string;
		services: AppService[];
	}
	interface TaskDetail {
		state: string;
		id: string;
		start_time: string;
		finish_time: string;
	}
	namespace App {
		// interface Error {}
		// interface Locals {}
		// interface PageData {}
		// interface PageState {}
		// interface Platform {}
	}
}

export {};
