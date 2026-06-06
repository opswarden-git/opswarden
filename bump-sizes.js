const fs = require('fs');
let content = fs.readFileSync('client-web/app/[locale]/settings/page.tsx', 'utf-8');

// Container
content = content.replace('max-w-4xl', 'max-w-6xl');

// Fonts
content = content.replaceAll('text-3xl', 'text-4xl');
content = content.replaceAll('text-lg', 'text-xl');
content = content.replaceAll('text-sm', 'text-base');
content = content.replaceAll('text-xs', 'text-sm');

// Icons
content = content.replaceAll('h-4 w-4', 'h-5 w-5');
content = content.replaceAll('h-5 w-5', 'h-6 w-6');
content = content.replaceAll('h-6 w-6', 'h-8 w-8');

// Specific paddings to make things breathe more
content = content.replaceAll('p-6', 'p-8');
content = content.replaceAll('p-3', 'p-4');

fs.writeFileSync('client-web/app/[locale]/settings/page.tsx', content);
console.log('Bumped sizes!');
