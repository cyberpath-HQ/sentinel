import { Accordion, AccordionItem, AccordionTrigger, AccordionContent } from "@/components/ui/accordion";
import { ChevronRight } from "lucide-react";

interface Doc {
    id: string;
    data: {
        title: string;
        subsection?: string;
    };
}

interface SidebarNavigationProps {
    sections: Record<string, Doc[]>;
    sortedSections: string[];
    currentSlug?: string;
}

/**
 * Client-side sidebar navigation with accordions and nested subsections
 */
export default function SidebarNavigation({ sections, sortedSections, currentSlug }: SidebarNavigationProps) {
    // Determine the current section based on currentSlug
    let currentSection = null;
    let currentSubsection = null;
    for (const section of sortedSections) {
        const doc = sections[section].find(doc => doc.id === currentSlug);
        if (doc) {
            currentSection = section;
            currentSubsection = doc.data.subsection;
            break;
        }
    }

    // Group docs by subsection within each section (only for docs that actually have subsections)
    const groupBySubsection = (docs: Doc[]): Record<string, Doc[]> => {
        return docs.reduce((acc, doc) => {
            if (doc.data.subsection) {
                if (!acc[doc.data.subsection]) {
                    acc[doc.data.subsection] = [];
                }
                acc[doc.data.subsection].push(doc);
            }
            return acc;
        }, {} as Record<string, Doc[]>);
    };

    // Check if a section has any docs with subsections
    const hasSubsections = (docs: Doc[]): boolean => {
        return docs.some(doc => doc.data.subsection);
    };

    // Get docs without subsections
    const getFlatDocs = (docs: Doc[]): Doc[] => {
        return docs.filter(doc => !doc.data.subsection);
    };

    return (
        <Accordion type="multiple" defaultValue={currentSection ? [currentSection] : []} className="space-y-2">
            {sortedSections.map((section) => {
                const docs = sections[section];
                const hasSubsections_ = hasSubsections(docs);
                const subsectionGroups = hasSubsections_ ? groupBySubsection(docs) : null;

                return (
                    <AccordionItem key={section} value={section} className="border-0">
                        <AccordionTrigger className="cursor-pointer py-2 px-2 text-xs font-semibold text-muted-foreground uppercase tracking-wider hover:no-underline">
                            {section}
                        </AccordionTrigger>
                        <AccordionContent>
                            {hasSubsections_ && subsectionGroups ? (
                                <>
                                    {/* Render flat docs first if any */}
                                    {(() => {
                                        const flatDocs = getFlatDocs(docs);
                                        return flatDocs.length > 0 ? (
                                            <ul className="space-y-1 mt-2">
                                                {flatDocs.map((doc) => {
                                                    const isActive = currentSlug === doc.id;
                                                    return (
                                                        <li key={doc.id}>
                                                            <a
                                                                href={`/docs/${doc.id}`}
                                                                className={`flex items-center gap-2 px-3 py-2 text-sm rounded-md transition-colors ${isActive
                                                                    ? "bg-primary/10 text-primary font-medium"
                                                                    : "text-muted-foreground hover:text-foreground hover:bg-muted"
                                                                    }`}
                                                            >
                                                                <ChevronRight className={`size-3 transition-transform`} />
                                                                {doc.data.title}
                                                            </a>
                                                        </li>
                                                    );
                                                })}
                                            </ul>
                                        ) : null;
                                    })()}
                                    {/* Render subsections */}
                                    <Accordion type="multiple" defaultValue={currentSubsection ? [currentSubsection] : []} className="space-y-1 mt-2">
                                        {Object.entries(subsectionGroups)
                                            .sort(([a], [b]) => a.localeCompare(b))
                                            .map(([subsection, subsectionDocs]) => (
                                                <AccordionItem key={subsection} value={subsection} className="border-0">
                                                    <AccordionTrigger className="cursor-pointer py-2 px-3 text-sm text-muted-foreground hover:no-underline">
                                                        <div className="flex items-center gap-x-2">
                                                            <ChevronRight className="size-3" />
                                                            {subsection}
                                                        </div>
                                                    </AccordionTrigger>
                                                    <AccordionContent>
                                                        <ul className="space-y-1 mt-2 ml-2">
                                                            {subsectionDocs.map((doc) => {
                                                                const isActive = currentSlug === doc.id;
                                                                return (
                                                                    <li key={doc.id}>
                                                                        <a
                                                                            href={`/docs/${doc.id}`}
                                                                            className={`flex items-center gap-2 px-3 py-2 text-sm rounded-md transition-colors ${isActive
                                                                                ? "bg-primary/10 text-primary font-medium"
                                                                                : "text-muted-foreground hover:text-foreground hover:bg-muted"
                                                                                }`}
                                                                        >
                                                                            <ChevronRight className={`size-3 transition-transform`} />
                                                                            {doc.data.title}
                                                                        </a>
                                                                    </li>
                                                                );
                                                            })}
                                                        </ul>
                                                    </AccordionContent>
                                                </AccordionItem>
                                            ))}
                                    </Accordion>
                                </>
                            ) : (
                                // Render flat list if no subsections
                                <ul className="space-y-1 mt-2">
                                    {docs.map((doc) => {
                                        const isActive = currentSlug === doc.id;
                                        return (
                                            <li key={doc.id}>
                                                <a
                                                    href={`/docs/${doc.id}`}
                                                    className={`flex items-center gap-2 px-3 py-2 text-sm rounded-md transition-colors ${isActive
                                                        ? "bg-primary/10 text-primary font-medium"
                                                        : "text-muted-foreground hover:text-foreground hover:bg-muted"
                                                        }`}
                                                >
                                                    <ChevronRight className={`size-3 transition-transform`} />
                                                    {doc.data.title}
                                                </a>
                                            </li>
                                        );
                                    })}
                                </ul>
                            )}
                        </AccordionContent>
                    </AccordionItem>
                );
            })}
        </Accordion>
    );
}
