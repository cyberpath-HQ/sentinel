import { useEffect, useState } from "react";

interface Heading {
    id: string;
    text: string;
    level: number;
}

interface TableOfContentsProps {
    headings?: Heading[];
}

/**
 * Floating Table of Contents component for documentation pages
 * Automatically detects headings in the page and provides jump links
 * Highlights the currently visible section
 */
export default function TableOfContents({ headings: propHeadings }: TableOfContentsProps) {
    const [headings, setHeadings] = useState<Heading[]>(propHeadings || []);
    const [activeId, setActiveId] = useState<string>("");

    useEffect(() => {
        // If headings weren't provided as props, extract them from the page
        if (!propHeadings || propHeadings.length === 0) {
            const elements = document.querySelectorAll("article h2, article h3, article h4");
            const extracted: Heading[] = [];

            elements.forEach((element) => {
                const id = element.id || element.textContent?.toLowerCase().replace(/\s+/g, "-") || "";
                if (!element.id && id) {
                    element.id = id;
                }

                extracted.push({
                    id: id,
                    text: element.textContent || "",
                    level: parseInt(element.tagName[1]),
                });
            });

            setHeadings(extracted);
        }
    }, [propHeadings]);

    useEffect(() => {
        // Track which heading is currently in view
        const observer = new IntersectionObserver(
            (entries) => {
                entries.forEach((entry) => {
                    if (entry.isIntersecting) {
                        setActiveId(entry.target.id);
                    }
                });
            },
            {
                rootMargin: "-100px 0px -66%",
                threshold: 1.0,
            }
        );

        headings.forEach(({ id }) => {
            const element = document.getElementById(id);
            if (element) {
                observer.observe(element);
            }
        });

        return () => {
            headings.forEach(({ id }) => {
                const element = document.getElementById(id);
                if (element) {
                    observer.unobserve(element);
                }
            });
        };
    }, [headings]);

    if (headings.length === 0) {
        return null;
    }

    const handleClick = (e: React.MouseEvent<HTMLAnchorElement>, id: string) => {
        e.preventDefault();
        const element = document.getElementById(id);
        if (element) {
            const offset = 80; // Account for sticky header
            const elementPosition = element.getBoundingClientRect().top;
            const offsetPosition = elementPosition + window.pageYOffset - offset;

            window.scrollTo({
                top: offsetPosition,
                behavior: "smooth",
            });
        }
    };

    return (
        <nav
            className="hidden xl:block fixed top-24 right-8 w-64 max-h-[calc(100vh-8rem)] overflow-y-auto overscroll-contain"
            aria-label="Table of contents"
        >
            <div className="sticky top-0 bg-background pb-2 z-10">
                <h4 className="text-sm font-semibold text-foreground mb-2">On This Page</h4>
            </div>
            <ul className="text-sm border-l-2 mb-4">
                {headings.map((heading) => {
                    const isActive = activeId === heading.id;
                    const indent = (heading.level - 2) * 16; // Indent based on heading level

                    return (
                        <li key={heading.id}>
                            <a
                                href={`#${heading.id}`}
                                onClick={(e) => handleClick(e, heading.id)}
                                className={`block py-1 border-l-4 pl-3 transition-colors ${isActive
                                    ? "border-primary text-primary font-medium"
                                    : "border-border text-muted-foreground hover:text-foreground hover:border-muted-foreground"
                                    }`}
                                style={{ paddingLeft: `${indent}px` }}
                            >
                                <span className="pl-3">
                                    {heading.text}
                                </span>
                            </a>
                        </li>
                    );
                })}
            </ul>
        </nav>
    );
}
