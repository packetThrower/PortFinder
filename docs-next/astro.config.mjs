// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

// https://astro.build/config
export default defineConfig({
	site: 'https://packetthrower.github.io',
	base: '/PortFinder/',
	trailingSlash: 'ignore',
	integrations: [
		starlight({
			title: 'PortFinder',
			description: 'Network switch port discovery via CDP, LLDP, and MNDP.',
			favicon: '/favicon.svg',
			customCss: ['@packetthrower/site-theme/styles/starlight.css'],
			components: {
				// Shared packetThrower header: global bar (Apps menu, search,
				// theme, GitHub) + PortFinder's app bar. The landing page at
				// /PortFinder/ is src/pages/index.astro, outside Starlight.
				Header: '@packetthrower/site-theme/components/StarlightHeader.astro',
			},
			editLink: {
				baseUrl: 'https://github.com/packetThrower/PortFinder/edit/main/docs-next/',
			},
			sidebar: [
				// Same groups, in the same order, on every packetThrower docs site.
				{
					label: 'Get started',
					items: [{ label: 'Install', slug: 'install' }],
				},
				{
					label: 'Guides',
					items: [
						{ label: 'GUI', slug: 'usage/gui' },
						{ label: 'CLI', slug: 'usage/cli' },
					],
				},
				{
					label: 'Develop',
					items: [
						{ label: 'Setup', slug: 'dev/setup' },
						{ label: 'Architecture', slug: 'dev/architecture' },
						{ label: 'Releasing', slug: 'dev/releasing' },
					],
				},
				{
					label: 'Project',
					items: [
						{ label: 'Changelog', slug: 'changelog' },
						{
							label: 'Issues',
							link: 'https://github.com/packetThrower/PortFinder/issues',
							attrs: { target: '_blank', rel: 'noopener' },
						},
					],
				},
			],
			lastUpdated: true,
		}),
	],
});
