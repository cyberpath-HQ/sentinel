import type { APIRoute } from "astro";
import { type CollectionEntry, getCollection } from "astro:content";
import { omit, title } from "radash";
import type { DocsMetadata, DocsMetadataCollection } from "@/lib/docs.ts";

/**
 * Get all the certifications metadata
 * @param {function} filter A function to filter the certifications
 * @returns {Promise<DocsMetadataCollection>}
 */
export async function getDocsMetadata(filter?: (entry: CollectionEntry<"docs">) => boolean): Promise<DocsMetadataCollection> {
    const docs = await getCollection(
        "docs",
        (entry) => {
            const reponse = !entry.data.draft;
            if (filter) {
                return reponse && filter(entry);
            }
            return reponse;
        },
    );

    return docs.map(
        (doc) =>
            ({
                ...omit(doc.data, [ "draft" ]),
                slug:     doc.id,
                id:       doc.id,
            }) as DocsMetadata,
    ) as DocsMetadataCollection;
}

/**
 * Dynamically generate a json with all the metadata from the docs
 */
export const GET: APIRoute = async () => {
    return Response.json(await getDocsMetadata());
};
