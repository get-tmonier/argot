const escapeHtml = (s: string): string =>
  s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');

/**
 * Render copy with `[[key phrases]]` emphasized (brighter). Everything else is
 * HTML-escaped, so inline code like `argot check` renders literally.
 */
export const emphasize = (text: string): string =>
  escapeHtml(text).replace(/\[\[(.+?)\]\]/g, '<strong class="font-medium text-ink">$1</strong>');
