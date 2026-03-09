import { cubicOut } from "svelte/easing";
import type { FadeParams, SlideParams, ScaleParams, FlyParams } from "svelte/transition";

export const defaultDuration = 200;

export const fadeConfig: FadeParams = {
  duration: defaultDuration,
  easing: cubicOut,
};

export const slideConfig: SlideParams = {
  duration: defaultDuration,
  easing: cubicOut,
};

export const scaleConfig: ScaleParams = {
  duration: defaultDuration,
  easing: cubicOut,
  start: 0.95,
};

export const flyConfig: FlyParams = {
  duration: defaultDuration,
  easing: cubicOut,
  y: 10,
};
