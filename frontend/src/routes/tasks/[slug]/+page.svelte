<script lang="ts">
	import CodeOutput from '../../../components/code-output.svelte';
	import PageHeader from '../../../components/page-header.svelte';
	import TaskStatusPill from '../../../components/task-status-pill.svelte';
	import TimeAgo from '../../../components/time-ago.svelte';
	import { tasks } from '../../../stores/tasksStore';
	import type { TaskDetail } from '../../../types';
	import { onMount } from 'svelte';
	import { setTitle } from '../../../stores/titleStore';

	export let data: TaskDetail;

	onMount(() => {
		setTitle(`Task: ${data.id} for ${data.app_name}`);
	});

	tasks.subscribe((new_tasks) => {
		data = Object.values(new_tasks).find((t) => t.id === data.id) || data;
	});
</script>

<PageHeader>
	<h2 class="card-title" slot="header">
		Task-Details for <br />{data.id}<br />
	</h2>
	<div slot="meta">
		<div class="flex items-center gap-2">
			<TaskStatusPill status={data.state} />
		</div>
		<div class="mt-2 text-xs text-gray-500">
			Started <TimeAgo dateString={data.start_time} /> <br />
			Finished <TimeAgo dateString={data.finish_time} /> <br />
			App: <a class="link-primary" href="/dashboard/{data.app_name}">{data.app_name}</a>
		</div>
	</div>
</PageHeader>
<CodeOutput heading="stdErr" output={data.stderr} />
<CodeOutput heading="stdOut" output={data.stdout} />
