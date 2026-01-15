import { useEffect, useState, useCallback } from "react";
import { X, Search, FileText, ChevronRight } from "lucide-react";
import Fuse, { type FuseResult, type RangeTuple } from "fuse.js";
import { all } from "radash";
import type { DocsMetadata, DocsMetadataCollection } from "@/lib/docs";
import { FuseConfig } from "@/pages/fuse.config";

interface SearchModalProps {
    isOpen: boolean;
    onClose?: () => void;
}

/**
 * Search modal with Fuse.js fuzzy search
 * Loads precomputed search index at build time for instant results
 */
export default function SearchModal({ isOpen: initialIsOpen, onClose }: SearchModalProps) {
    const [isOpen, setIsOpen] = useState(initialIsOpen);
    const [query, setQuery] = useState("");
    const [results, setResults] = useState<Array<FuseResult<DocsMetadata>>>([]);
    const [fuse, setFuse] = useState<Fuse<DocsMetadata> | null>(null);
    const [loading, setLoading] = useState(true);
    const [selectedIndex, setSelectedIndex] = useState(0);

    // Listen for toggle events from the page
    useEffect(() => {
        const handleToggle = (e: CustomEvent) => {
            setIsOpen(e.detail.isOpen);
        };

        window.addEventListener("toggle-search", handleToggle as EventListener);
        return () => window.removeEventListener("toggle-search", handleToggle as EventListener);
    }, []);

    const handleClose = useCallback(() => {
        setIsOpen(false);
        window.dispatchEvent(new CustomEvent("close-search"));
        if (typeof onClose === 'function') {
            onClose();
        }
    }, [onClose]);

    // Load search index when modal opens
    useEffect(() => {
        if (isOpen && !fuse) {
            all({
                data: fetch(`/data.json`),
                index: fetch(`/data-index.json`),
            })
                .then(async ({ data, index }) => {
                    const all_data: DocsMetadataCollection = await data.json();
                    const parsed_index = Fuse.parseIndex<DocsMetadata>(await index.json());
                    const fuseInstance = new Fuse(
                        all_data,
                        FuseConfig,
                        parsed_index,
                    );
                    setFuse(fuseInstance);
                    setLoading(false);
                })
                .catch((error) => {
                    console.error("Failed to load search index:", error);
                    setLoading(false);
                });
        }
    }, [isOpen, fuse]);

    // Perform search when query changes
    useEffect(() => {
        if (!fuse || !query.trim()) {
            setResults([]);
            setSelectedIndex(0);
            return;
        }

        const searchResults = fuse.search(query, { limit: 10 });
        setResults(searchResults);
        setSelectedIndex(0);
    }, [query, fuse]);

    // Handle keyboard navigation
    useEffect(() => {
        const handleKeyDown = (e: KeyboardEvent) => {
            if (!isOpen) return;

            if (e.key === "Escape") {
                handleClose();
            } else if (e.key === "ArrowDown") {
                e.preventDefault();
                setSelectedIndex((prev) => (prev < results.length - 1 ? prev + 1 : prev));
            } else if (e.key === "ArrowUp") {
                e.preventDefault();
                setSelectedIndex((prev) => (prev > 0 ? prev - 1 : 0));
            } else if (e.key === "Enter" && results[selectedIndex]) {
                e.preventDefault();
                navigateToResult(results[selectedIndex].item);
            }
        };

        document.addEventListener("keydown", handleKeyDown);
        return () => document.removeEventListener("keydown", handleKeyDown);
    }, [isOpen, results, selectedIndex, handleClose]);

    // Close modal on backdrop click
    const handleBackdropClick = useCallback(
        (e: React.MouseEvent) => {
            if (e.target === e.currentTarget) {
                handleClose();
            }
        },
        [handleClose]
    );

    const navigateToResult = (result: DocsMetadata) => {
        window.location.href = `/docs/${result.slug}`;
        handleClose();
    };

    const highlightMatch = (text: string, matches?: ReadonlyArray<readonly [number, number]>) => {
        if (!matches || matches.length === 0) return text;

        const parts: React.ReactElement[] = [];
        let lastIndex = 0;

        matches.forEach(([start, end], i) => {
            if (start > lastIndex) {
                parts.push(<span key={`text-${i}`}>{text.slice(lastIndex, start)}</span>);
            }
            parts.push(
                <mark key={`match-${i}`} className="bg-primary/20 text-primary">
                    {text.slice(start, end + 1)}
                </mark>
            );
            lastIndex = end + 1;
        });

        if (lastIndex < text.length) {
            parts.push(<span key="text-end">{text.slice(lastIndex)}</span>);
        }

        return <>{parts}</>;
    };

    if (!isOpen) return null;

    return (
        <div
            className="fixed inset-0 z-100 bg-background/80 backdrop-blur-sm flex items-start justify-center pt-20 sm:pt-20 px-4"
            onClick={handleBackdropClick}
        >
            <div className="w-full max-w-2xl bg-card border border-border rounded-lg shadow-2xl overflow-hidden animate-in fade-in zoom-in-95 duration-200 max-h-[85vh] sm:max-h-[70vh] flex flex-col">
                {/* Search Input */}
                <div className="flex items-center gap-3 px-4 py-3 border-b border-border shrink-0">
                    <Search className="h-5 w-5 text-muted-foreground" />
                    <input
                        type="text"
                        placeholder="Search documentation..."
                        value={query}
                        onChange={(e) => setQuery(e.target.value)}
                        className="flex-1 bg-transparent text-base outline-none placeholder:text-muted-foreground"
                        autoFocus
                    />
                    <button
                        onClick={handleClose}
                        className="p-1 hover:bg-muted rounded-md transition-colors"
                        aria-label="Close search"
                    >
                        <X className="h-4 w-4" />
                    </button>
                </div>

                {/* Results */}
                <div className="flex-1 overflow-y-auto overscroll-contain">
                    {loading ? (
                        <div className="flex items-center justify-center py-12 text-muted-foreground">
                            <div className="animate-spin rounded-full h-8 w-8 border-2 border-primary border-t-transparent"></div>
                        </div>
                    ) : query.trim() && results.length === 0 ? (
                        <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
                            <Search className="h-12 w-12 mb-4 opacity-20" />
                            <p className="text-sm">No results found for "{query}"</p>
                        </div>
                    ) : query.trim() && results.length > 0 ? (
                        <ul className="py-2">
                            {results.map((result, index) => {
                                const titleMatch = result.matches?.find((m: any) => m.key === "title");
                                const descriptionMatch = result.matches?.find((m: any) => m.key === "description");
                                const contentMatch = result.matches?.find((m: any) => m.key === "content");

                                return (
                                    <li key={result.item.slug}>
                                        <button
                                            onClick={() => navigateToResult(result.item)}
                                            className={`w-full text-left px-4 py-3 hover:bg-muted transition-colors ${index === selectedIndex ? "bg-muted" : ""
                                                }`}
                                            onMouseEnter={() => setSelectedIndex(index)}
                                        >
                                            <div className="flex items-start gap-3">
                                                <FileText className="h-5 w-5 text-primary mt-0.5 shrink-0" />
                                                <div className="flex-1 min-w-0">
                                                    <div className="flex items-center gap-2 mb-1">
                                                        <h3 className="text-sm font-medium">
                                                            {titleMatch?.indices
                                                                ? highlightMatch(result.item.title, titleMatch.indices)
                                                                : result.item.title}
                                                        </h3>
                                                        <span className="text-xs text-muted-foreground px-2 py-0.5 bg-muted rounded-full shrink-0">
                                                            {result.item.section}
                                                        </span>
                                                    </div>
                                                    <p className="text-xs text-muted-foreground line-clamp-2">
                                                        {descriptionMatch?.indices
                                                            ? highlightMatch(result.item.description, descriptionMatch.indices)
                                                            : contentMatch?.indices
                                                                ? highlightMatch(
                                                                    result.item.description.slice(0, 150) + "...",
                                                                    contentMatch.indices
                                                                )
                                                                : result.item.description}
                                                    </p>
                                                </div>
                                                <ChevronRight className="h-4 w-4 text-muted-foreground shrink-0 mt-0.5" />
                                            </div>
                                        </button>
                                    </li>
                                );
                            })}
                        </ul>
                    ) : (
                        <div className="py-12 px-4 text-center">
                            <p className="text-sm text-muted-foreground mb-4">Start typing to search documentation</p>
                            <div className="flex items-center justify-center gap-6 text-xs text-muted-foreground">
                                <div className="flex items-center gap-2">
                                    <kbd className="px-2 py-1 bg-muted border border-border rounded">↑↓</kbd>
                                    <span>Navigate</span>
                                </div>
                                <div className="flex items-center gap-2">
                                    <kbd className="px-2 py-1 bg-muted border border-border rounded">Enter</kbd>
                                    <span>Select</span>
                                </div>
                                <div className="flex items-center gap-2">
                                    <kbd className="px-2 py-1 bg-muted border border-border rounded">Esc</kbd>
                                    <span>Close</span>
                                </div>
                            </div>
                        </div>
                    )}
                </div>
            </div>
        </div>
    );
}
