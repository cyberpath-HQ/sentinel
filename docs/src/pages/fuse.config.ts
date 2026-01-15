import type { DocsMetadata } from "@/lib/docs";
import type { IFuseOptions } from "fuse.js";

export const FuseConfig: IFuseOptions<DocsMetadata> = {
    keys:              [
        {
            name:   "title",
            weight: 0.7,
        },
        {
            name:   "description",
            weight: 0.6,
        },
        {
            name:   "related",
            weight: 0.5,
        },
        {
            name:   "keywords",
            weight: 0.4,
        },
        {
            name:   "section",
            weight: 0.3,
        },
    ],
    isCaseSensitive:   false,
    useExtendedSearch: true,
    threshold: 0.4,
    includeScore: true,
    includeMatches: true,
    minMatchCharLength: 2,
    ignoreLocation: true,
};