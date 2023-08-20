import "vite/modulepreload-polyfill";
import htmx from "htmx.org";

declare module "htmx.org" {
	export interface HtmxConfig {
		/**
		 * If set to `true`, htmx will use the
		 * [View Transition API](https://developer.mozilla.org/en-US/docs/Web/API/View_Transitions_API)
		 * when swapping in new content.
		 */
		globalViewTransitions: boolean;
	}
}

htmx.config.allowEval = false;
htmx.config.useTemplateFragments = true;
htmx.config.globalViewTransitions = true;
// htmx.logAll();

// htmx.on("htmx:afterProcessNode", (e) => {
// 	const { target } = e as { target: HTMLElement | null };
// 	if (target) {
// 		const viewName = target.getAttribute("hx-view-name");
// 		if (viewName) {
// 			(target.style as any).viewTransitionName = viewName;
// 		}
// 	}
// });
