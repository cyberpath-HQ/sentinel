/**
 * Header Component for Sentinel Documentation
 *
 * Modern header with navigation, search, and mobile menu
 */

import { useState, useEffect, type FC } from "react";
import { Button } from "@/components/ui/button";
import { Sheet, SheetContent, SheetTrigger } from "@/components/ui/sheet";
import { cn } from "@/lib/utils";
import { Menu, ExternalLink, Search, PanelRightClose } from "lucide-react";

interface HeaderProps {
    activePage?: "home" | "docs";
}

export const SiteHeader: FC<HeaderProps> = ({ activePage = "home" }) => {
    const [isOpen, setIsOpen] = useState(false);

    const navLinks = [
        { href: "/", label: "Home", page: "home" as const },
        { href: "/docs/introduction", label: "Documentation", page: "docs" as const },
    ];

    // Handle keyboard shortcut for search
    useEffect(() => {
        const handleKeyDown = (e: KeyboardEvent) => {
            if ((e.metaKey || e.ctrlKey) && e.key === "k") {
                e.preventDefault();
                window.dispatchEvent(new CustomEvent("toggle-search", { detail: { isOpen: true } }));
            }
        };

        document.addEventListener("keydown", handleKeyDown);
        return () => document.removeEventListener("keydown", handleKeyDown);
    }, []);

    // Handle mobile sidebar toggle for docs pages
    useEffect(() => {
        if (activePage === "docs") {
            const handleMenuToggle = () => {
                const sidebar = document.getElementById("sidebar");
                const overlay = document.getElementById("sidebar-overlay");
                if (sidebar && overlay) {
                    sidebar.classList.toggle("-translate-x-full");
                    overlay.classList.toggle("hidden");
                }
            };

            // Store the handler so we can clean it up if needed
            (window as any).__docsMenuToggle = handleMenuToggle;
        }
    }, [activePage]);

    const openSearch = () => {
        setIsOpen(false);
        window.dispatchEvent(new CustomEvent("toggle-search", { detail: { isOpen: true } }));
    };

    return (
        <header className="sticky top-0 z-50 w-full border-b border-border bg-background/95 backdrop-blur supports-backdrop-filter:bg-background/60">
            <div className={cn("mx-auto flex h-16 items-center justify-between px-4 lg:px-8", activePage !== "docs" && "container")}>
                {/* Mobile Sidebar Toggle (docs only) */}
                {activePage === "docs" && (
                    <button
                        id="mobile-menu-toggle"
                        onClick={() => {
                            const sidebar = document.getElementById("sidebar");
                            const overlay = document.getElementById("sidebar-overlay");
                            sidebar?.classList.toggle("-translate-x-full");
                            overlay?.classList.toggle("hidden");
                        }}
                        className="lg:hidden p-2 hover:bg-muted rounded-md transition-colors"
                        aria-label="Toggle navigation"
                    >
                        <PanelRightClose className="size-4" />
                    </button>
                )}

                {/* Logo */}
                <a href="/" className="flex items-center gap-3 hover:opacity-80 transition-opacity">
                    <img src={"/logo-white.svg"} alt="Sentinel Logo" className={"h-8 max-h-8 w-auto"} />
                </a>

                {/* Desktop Navigation */}
                <nav className="hidden md:flex items-center gap-8 flex-1 ml-8">
                    {navLinks.map((link) => (
                        <a
                            key={link.href}
                            href={link.href}
                            className={cn(
                                "text-sm font-medium transition-colors hover:text-primary",
                                activePage === link.page ? "text-foreground" : "text-muted-foreground"
                            )}
                        >
                            {link.label}
                        </a>
                    ))}
                    <a
                        href="https://github.com/cyberpath-HQ/sentinel"
                        target="_blank"
                        rel="noopener noreferrer"
                        className="text-sm font-medium text-muted-foreground hover:text-foreground transition-colors flex items-center gap-1.5"
                    >
                        GitHub
                    </a>
                </nav>

                {/* Right side actions */}
                <div className="flex items-center gap-4 ml-auto">

                    {/* Desktop Search Button (docs only) */}
                    {activePage === "docs" && (
                        <button
                            id="search-button"
                            onClick={openSearch}
                            className="hidden lg:flex items-center gap-2 px-3 py-1.5 text-sm text-muted-foreground bg-muted/50 border border-border rounded-lg hover:bg-muted transition-colors"
                        >
                            <Search className="h-4 w-4" />
                            <span>Search docs...</span>
                            <kbd className="ml-4 px-1.5 py-0.5 text-xs bg-background border border-border rounded">⌘K</kbd>
                        </button>
                    )}

                    {/* Desktop CTA (home only) */}
                    {activePage === "home" && (
                        <div className="hidden md:flex items-center gap-4">
                            <Button asChild variant="default" size="sm" className="gap-2">
                                <a href="/docs/introduction">
                                    Get Started
                                    <ExternalLink className="h-3.5 w-3.5" />
                                </a>
                            </Button>
                        </div>
                    )}

                    {/* Mobile Menu */}
                    <Sheet open={isOpen} onOpenChange={setIsOpen}>
                        <SheetTrigger asChild className="md:hidden">
                            <Button variant="ghost" size="icon">
                                <Menu className="h-5 w-5" />
                                <span className="sr-only">Toggle menu</span>
                            </Button>
                        </SheetTrigger>
                        <SheetContent side="right" className="w-72 bg-sidebar border-l border-border p-4">
                            <div className="flex flex-col gap-6 mt-8">
                                <nav className="flex flex-col gap-4">
                                    {navLinks.map((link) => (
                                        <a
                                            key={link.href}
                                            href={link.href}
                                            onClick={() => setIsOpen(false)}
                                            className={cn(
                                                "text-base font-medium transition-colors hover:text-primary py-2",
                                                activePage === link.page ? "text-foreground" : "text-muted-foreground"
                                            )}
                                        >
                                            {link.label}
                                        </a>
                                    ))}
                                    <a
                                        href="https://github.com/cyberpath-HQ/sentinel"
                                        target="_blank"
                                        rel="noopener noreferrer"
                                        onClick={() => setIsOpen(false)}
                                        className="text-base font-medium text-muted-foreground hover:text-foreground transition-colors flex items-center gap-2 py-2"
                                    >
                                        GitHub
                                    </a>
                                </nav>
                                {activePage === "home" && (
                                    <div className="pt-4 border-t border-border">
                                        <Button asChild className="w-full gap-2">
                                            <a href="/docs/introduction">
                                                Get Started
                                                <ExternalLink className="h-3.5 w-3.5" />
                                            </a>
                                        </Button>
                                    </div>
                                )}
                                {activePage === "docs" && (
                                    <div className="pt-4 border-t border-border flex justify-center w-full">
                                        <button
                                            id="search-button"
                                            onClick={openSearch}
                                            className="w-full flex items-center gap-2 px-3 py-1.5 text-sm text-muted-foreground bg-muted/50 border border-border rounded-lg hover:bg-muted transition-colors"
                                        >
                                            <Search className="h-4 w-4" />
                                            <span>Search docs...</span>
                                        </button>
                                    </div>
                                )}
                            </div>
                        </SheetContent>
                    </Sheet>
                </div>
            </div>
        </header>
    );
};

export default SiteHeader;

