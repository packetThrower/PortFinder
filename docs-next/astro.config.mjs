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
			logo: {
				src: './src/assets/icon.svg',
				replacesTitle: false,
			},
			favicon: '/favicon.svg',
			customCss: ['./src/styles/theme.css'],
			components: {
				Hero: './src/components/Hero.astro',
			},
			social: [
				{
					icon: 'github',
					label: 'GitHub',
					href: 'https://github.com/packetThrower/PortFinder',
				},
			],
			editLink: {
				baseUrl: 'https://github.com/packetThrower/PortFinder/edit/main/docs-next/src/content/docs/',
			},
			sidebar: [
				{ label: 'Install', slug: 'install' },
				{
					label: 'Usage',
					items: [
						{ label: 'GUI', slug: 'usage/gui' },
						{ label: 'CLI', slug: 'usage/cli' },
					],
				},
				{
					label: 'Development',
					items: [
						{ label: 'Setup', slug: 'dev/setup' },
						{ label: 'Architecture', slug: 'dev/architecture' },
						{ label: 'Releasing', slug: 'dev/releasing' },
					],
				},
				{ label: 'Changelog', slug: 'changelog' },
			],
			lastUpdated: true,
		}),
	],
});
