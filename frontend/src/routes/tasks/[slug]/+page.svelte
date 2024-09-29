<script lang="ts">
	import CodeOutput from '../../../components/code-output.svelte';
	import PageHeader from '../../../components/page-header.svelte';
	import TaskStatusPill from '../../../components/task-status-pill.svelte';
	import TimeAgo from '../../../components/time-ago.svelte';
	import { tasks } from '../../../stores/tasksStore';
	import type { TaskDetail } from '../../../types';

	export let data: TaskDetail;

	tasks.subscribe((new_tasks) => {
		console.log(new_tasks, data.id);
		data = Object.values(new_tasks).find((t) => t.id === data.id) || data;
	});
</script>

<PageHeader>
	<h2 class="card-title" slot="header">Task-Details for <br />{data.id}</h2>
	<div slot="meta">
		<TaskStatusPill status={data.state} /> <br />
		<div class="mt-2 text-xs text-gray-500">
			Started: <TimeAgo dateString={data.start_time} /> <br />
			Finished: <TimeAgo dateString={data.finish_time} /> <br />
		</div>
	</div>
</PageHeader>
<CodeOutput heading="stdErr" output={data.stderr} />
<CodeOutput heading="stdOut" output={data.stdout} />
