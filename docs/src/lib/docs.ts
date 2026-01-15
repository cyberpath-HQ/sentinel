import type { CollectionEntry } from "astro:content";

export type DocsMetadata = CollectionEntry<"docs">["data"] & {
    title: string;
    description: string;
    section: string;
    order: number;
    keywords: string[];
    related: string[];
    slug: string;
    id: string;
}

export type DocsMetadataCollection = DocsMetadata[];