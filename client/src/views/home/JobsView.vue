<script setup lang="ts">
import { inject } from "vue";
import { jobServiceInjectionToken } from "@/services/jobs-service";
import JobInfo from "@/components/jobs/JobInfo.vue";

const jobService = inject(jobServiceInjectionToken)!;
</script>

<template>
    <div class="jobs-view">
        <div v-if="!jobService.jobsCount.value">No active jobs.</div>
        <div v-else>
            <div class="jobs-view__title">Running {{ jobService.jobsCount }} jobs</div>
            <div v-for="(job, index) in jobService.getJobs()" class="job-view__item" :key="job.id">
                <job-info :job="job" :index="index" />
            </div>
        </div>
    </div>
</template>

<style>
.jobs-view {
    padding: 15px;
}
.jobs-view__title {
    font-size: 1.4rem;
    padding-bottom: 20px;
}

.job-view__item {
    padding: 15px 0;
}

.job-view__item:not(:last-child) {
    border-bottom: 1px solid var(--border-color);
}
</style>
