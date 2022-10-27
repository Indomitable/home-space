<script setup lang="ts">
import type { Job } from "@/services/jobs-service";
import { inject } from "vue";
import { jobServiceInjectionToken } from "@/services/jobs-service";

export interface JobInfoProps {
    job: Job;
    index: number;
}

const props = defineProps<JobInfoProps>();
const jobService = inject(jobServiceInjectionToken)!;
const execution = jobService.getJobExecution(props.job.id)!;
</script>

<template>
    <div class="job-info-components">
        <div class="job-info__title">{{ index + 1 }}. {{ job.name }}</div>
        <div class="job-info__info">
            {{ execution.info }}
            <span class="job-info__progress" v-if="job.steps"> {{ execution.progress || 0 }} of {{ job.steps }} </span>
        </div>
    </div>
</template>
<style>
.job-info__title {
    font-size: 1.1rem;
    padding-bottom: 10px;
}

.job-info__progress {
    font-size: 1rem;
}
</style>
