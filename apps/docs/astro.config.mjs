// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

export default defineConfig({
  integrations: [
    starlight({
      title: 'Starbase',
      sidebar: [
        {
          label: 'Home',
          items: [
            { label: 'Welcome', slug: '' },
          ],
        },
        {
          label: 'Architecture',
          items: [
            { label: 'Architecture Overview', slug: 'architecture' },
          ],
        },
        {
          label: 'CLI Reference',
          items: [
            { label: 'starbase CLI', slug: 'cli' },
          ],
        },
        {
          label: 'Reference',
          items: [
            { label: 'Schema', slug: 'schema' },
          ],
        },
      ],
    }),
  ],
});
