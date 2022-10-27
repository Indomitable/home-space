<script setup lang="ts">
import { inject } from "vue";
import { jobServiceInjectionToken } from "@/services/jobs-service";

const jobService = inject(jobServiceInjectionToken)!;
const count = jobService.jobsCount;
</script>

<template>
    <div v-if="count > 0" class="job-notification">
        <router-link :to="{ name: 'jobs' }" class="job-notification-link">
            <span>{{ count }}</span>
        </router-link>
    </div>
</template>

<style>
.job-notification {
    position: absolute;
    bottom: 20px;
    right: 20px;
    border-radius: 25px;
    border: 1px solid var(--border-color);
    background: var(--background-color);
    width: 50px;
    height: 50px;
    font-size: 1.3rem;
}

.job-notification-link {
    color: var(--primary-font-color);
    text-decoration: none;
    display: flex;
    height: 100%;
    width: 100%;
    border-radius: 25px;
    flex-direction: column;
    align-items: center;
    justify-content: center;
}
</style>
