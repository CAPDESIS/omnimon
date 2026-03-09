import { writable } from "svelte/store";
import type { ProcessEntry } from "../lib/types";

/** Process to show in the details modal, set from anywhere (e.g., AI chat) */
export const inspectProcessRequest = writable<ProcessEntry | null>(null);

/** Request to show the Chrome tab manager */
export const showTabManagerRequest = writable<boolean>(false);

/** Request to send a prompt to the AI chat */
export const askAiRequest = writable<string | null>(null);
