function configure(control: HTMLInputElement | HTMLTextAreaElement) {
  control.setAttribute("autocorrect", "off");
  control.setAttribute("autocapitalize", "none");
  control.setAttribute("autocomplete", "off");
  control.spellcheck = false;
}

function configureTree(root: ParentNode) {
  if (root instanceof HTMLInputElement || root instanceof HTMLTextAreaElement) configure(root);
  root.querySelectorAll<HTMLInputElement | HTMLTextAreaElement>("input, textarea").forEach(configure);
}

export function installTextInputPolicy(root: Document = document): () => void {
  configureTree(root);
  const observer = new MutationObserver((mutations) => {
    for (const mutation of mutations) {
      mutation.addedNodes.forEach((node) => {
        if (node instanceof HTMLElement) configureTree(node);
      });
    }
  });
  observer.observe(root.documentElement, { childList: true, subtree: true });
  return () => observer.disconnect();
}
