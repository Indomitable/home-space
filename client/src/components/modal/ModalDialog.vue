<script setup lang="ts">
interface ModalDialogProps {
    header?: string;
    showBackdrop: boolean;
}

interface ModalDialogEvents {
    (event: "cancel"): void;
}
defineProps<ModalDialogProps>();
defineEmits<ModalDialogEvents>();
</script>

<template>
    <Teleport to="body">
        <div class="modal-dialog popup">
            <div class="modal-header">
                <slot name="header">{{ header }}</slot>
            </div>
            <div class="modal-body">
                <slot></slot>
            </div>
        </div>
        <div v-if="showBackdrop" class="modal-backdrop" @click="$emit('cancel')"></div>
    </Teleport>
</template>

<style>
.modal-dialog {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translateX(-50%) translateY(-50%);
    z-index: var(--z-index-modal-dialog);
    background: var(--background-color);
    width: 50vw;
    height: 50vh;
    display: flex;
    flex-direction: column;
    padding: 20px;
}

.modal-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.4);
    z-index: var(--z-index-modal-dialog-backdrop);
}

.modal-header {
    padding: 10px 0;
    font-size: 17px;
}

.modal-body {
    flex: 1 0;
}
</style>
