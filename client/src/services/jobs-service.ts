import type { InjectionKey } from "vue";
import { reactive, ref } from "vue";

export interface Job {
    id: number;
    name: string;
    steps?: number;
}

export interface JobExecution {
    info: string;
    progress?: number;
}

export class JobService {
    private jobId = 0;
    private readonly jobs: Record<number, Job>;
    private readonly executions: Record<number, JobExecution>;
    jobsCount = ref(0);

    constructor() {
        this.jobs = reactive({});
        this.executions = reactive({});
    }

    public addJob(job: Job): number {
        this.jobId = this.jobId + 1;
        job.id = this.jobId;
        this.jobs[this.jobId] = job;
        this.executions[this.jobId] = {
            info: "",
        };
        this.jobsCount.value = this.jobsCount.value + 1;
        return this.jobId;
    }

    public reportProgress(id: number, executingStep: number) {
        const execution = this.executions[id];
        if (execution) {
            execution.progress = executingStep;
        }
    }

    public setInfo(id: number, message: string) {
        const execution = this.executions[id];
        if (execution) {
            execution.info = message;
        }
    }

    public getJobs(): Job[] {
        return Array.from(Object.values(this.jobs));
    }

    public getJobExecution(jobId: number): JobExecution | undefined {
        return this.executions[jobId];
    }

    public finishJob(id: number) {
        delete this.jobs[id];
        this.jobsCount.value = this.jobsCount.value - 1;
    }
}

export const jobServiceInjectionToken: InjectionKey<JobService> = Symbol("JobService");
