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

declare global {
	export interface CSSStyleDeclaration {
		viewTransitionName: string;
	}
}

htmx.config.allowEval = false;
htmx.config.useTemplateFragments = true;
htmx.config.globalViewTransitions = true;
// htmx.logAll();

htmx.on("htmx:beforeTransition", (e) => {
	const evt = e as CustomEvent<{
		readonly boosted: boolean;
	}> & { target: HTMLElement };
	const { target } = evt;
	if (!target) return;
	if (!evt.detail.boosted) return;

	const viewTransitionItem = target.closest<HTMLElement>(
		"[hx-view-transition-name]"
	);
	if (!viewTransitionItem) return;

	const viewTransitionName = viewTransitionItem.getAttribute(
		"hx-view-transition-name"
	)!;
	// console.log(
	// 	`Found view transition item (${viewTransitionName}):`,
	// 	viewTransitionItem
	// );
	viewTransitionItem.style.viewTransitionName = viewTransitionName;
});
// htmx.on("htmx:afterProcessNode", (e) => {
// 	const { target } = e as { target: HTMLElement | null };
// 	if (target) {
// 		const viewName = target.getAttribute("hx-view-name");
// 		if (viewName) {
// 			(target.style as any).viewTransitionName = viewName;
// 		}
// 	}
// });
