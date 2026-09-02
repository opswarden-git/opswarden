localStorage.removeItem('__palette');
document.body.setAttribute('data-md-color-scheme', 'slate');

document.addEventListener('keydown', (event) => {
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k') {
    event.preventDefault();
    document.querySelector('[data-md-toggle="search"]')?.click();
    window.setTimeout(() => document.querySelector('.md-search__input')?.focus(), 0);
  }
});

// Material s'arrête au parent : le fil d'Ariane n'indique jamais où l'on est.
// On ajoute la page courante en dernier maillon, non cliquable.
(() => {
  const path = document.querySelector('.md-path__list');
  const title = document.querySelector('.md-typeset h1');
  if (!path || !title || !path.children.length) return;
  const label = title.textContent.replace(/¶/g, '').trim();
  if (!label) return;
  const last = path.lastElementChild.textContent.trim();
  if (last === label) return;
  const item = document.createElement('li');
  item.className = 'md-path__item md-path__item--current';
  item.setAttribute('aria-current', 'page');
  const span = document.createElement('span');
  span.className = 'md-path__link md-path__link--current';
  span.textContent = label;
  item.append(span);
  path.append(item);
})();

// Material 9.7.7 remplace le <pre class="mermaid"> par un <div> vide : son
// appel à mermaid.render() réussit, mais le SVG n'est jamais inséré. On garde
// la source avant qu'elle disparaisse et on rend nous-mêmes les conteneurs
// restés vides. Si l'intégration se remet à fonctionner, ce code ne fait rien.
(() => {
  const sources = [...document.querySelectorAll('pre.mermaid, div.mermaid')]
    .map((element) => element.textContent.trim())
    .filter(Boolean);
  if (!sources.length) return;

  const isEmpty = (element) => !element.querySelector('svg') && !element.textContent.trim();

  const fill = async () => {
    const targets = [...document.querySelectorAll('pre.mermaid, div.mermaid')];
    if (targets.length !== sources.length) return false;
    const pending = targets.filter(isEmpty);
    if (!pending.length) return true;
    for (const [index, target] of targets.entries()) {
      if (!isEmpty(target)) continue;
      try {
        const { svg, bindFunctions } = await window.mermaid.render(`ops-mermaid-${index}`, sources[index]);
        target.innerHTML = svg;
        bindFunctions?.(target);
        // Le SVG sort en width:100% sans hauteur : sans dimensions explicites
        // il se rend à zéro. On les reprend du viewBox.
        const drawn = target.querySelector('svg');
        const box = drawn?.getAttribute('viewBox')?.split(/\s+/).map(Number);
        if (box && box.length === 4 && box[2] > 0) {
          drawn.setAttribute('width', String(box[2]));
          drawn.setAttribute('height', String(box[3]));
          drawn.style.maxWidth = '100%';
          drawn.style.height = 'auto';
        }
      } catch {
        target.textContent = sources[index];
      }
    }
    return true;
  };

  let tries = 0;
  const tick = async () => {
    tries += 1;
    if (window.mermaid && (await fill())) return;
    if (tries < 30) window.setTimeout(tick, 150);
  };
  window.setTimeout(tick, 150);
})();
