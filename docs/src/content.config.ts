import { defineCollection, z } from "astro:content";
import { glob } from "astro/loaders";

/**
 * Documentation collection schema
 * Defines the structure and validation for documentation pages
 */
const docsCollection = defineCollection({
    loader: glob({ pattern: "**/*.{md,mdx}", base: "./src/content/docs" }),
    schema: z.object({
        // Page title displayed in browser tab and page header
        title: z.string(),
        // Short description for SEO and page previews
        description: z.string(),
        // Sidebar section grouping (e.g., "Getting Started", "Core Concepts")
        section: z.string().default("General"),
        // Optional subsection for nested grouping within a section (e.g., "WAL" under "Advanced Topics")
        subsection: z.string().optional(),
        // Order within the section (lower numbers appear first)
        order: z.number().default(100),
        // Whether this page appears in the sidebar navigation
        sidebar: z.boolean().default(true),
        // Optional icon name from lucide-react for sidebar display
        icon: z.string().optional(),
        // Last updated date for the page
        lastUpdated: z.date().optional(),
        // Draft pages are hidden in production
        draft: z.boolean().default(false),
        // Related pages for "See Also" section
        related: z.array(z.string()).optional(),
        // Keywords for search functionality
        keywords: z.array(z.string()).optional(),
    }),
});

export const collections = {
    docs: docsCollection,
};
