import { Accordion, AccordionItem, AccordionTrigger, AccordionContent } from "@/components/ui/accordion";
import { ChevronRight } from "lucide-react";

interface Doc {
    id: string;
    data: {
        title: string;
    };
}

interface SidebarNavigationProps {
    sections: Record<string, Doc[]>;
    sortedSections: string[];
    currentSlug?: string;
}

/**
 * Client-side sidebar navigation with accordions
 */
export default function SidebarNavigation({ sections, sortedSections, currentSlug }: SidebarNavigationProps) {
    return (
        <Accordion type="multiple" defaultValue={sortedSections} className="space-y-2">
            {sortedSections.map((section) => (
                <AccordionItem key={section} value={section} className="border-0">
                    <AccordionTrigger className="cursor-pointer py-2 px-2 text-xs font-semibold text-muted-foreground uppercase tracking-wider hover:no-underline">
                        {section}
                    </AccordionTrigger>
                    <AccordionContent>
                        <ul className="space-y-1 mt-2">
                            {sections[section].map((doc) => {
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
                                            <ChevronRight className={`h-3 w-3 transition-transform`} />
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
    );
}
