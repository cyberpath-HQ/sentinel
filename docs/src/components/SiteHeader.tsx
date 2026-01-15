/**
 * Header Component for Sentinel Documentation
 *
 * Modern header with navigation, search, and mobile menu
 */

import { useState, type FC } from "react";
import { Button } from "@/components/ui/button";
import { Sheet, SheetContent, SheetTrigger } from "@/components/ui/sheet";
import { cn } from "@/lib/utils";
import { Menu, Shield, Github, ExternalLink } from "lucide-react";

interface HeaderProps {
    activePage?: "home" | "docs";
}

export const SiteHeader: FC<HeaderProps> = ({ activePage = "home" }) => {
    const [isOpen, setIsOpen] = useState(false);

    const navLinks = [
        { href: "/", label: "Home", page: "home" as const },
        { href: "/docs/introduction", label: "Documentation", page: "docs" as const },
    ];

    return (
        <header className="sticky top-0 z-50 w-full border-b border-border bg-background/95 backdrop-blur supports-backdrop-filter:bg-background/60">
            <div className="container mx-auto flex h-16 items-center justify-between px-4 lg:px-8">
                {/* Logo */}
                <a href="/" className="flex items-center gap-3 hover:opacity-80 transition-opacity">
                    <div className="flex items-center justify-center w-9 h-9 rounded-lg bg-primary/10 border border-primary/20">
                        <Shield className="h-5 w-5 text-primary" />
                    </div>
                    <div className="flex flex-col">
                        <span className="font-bold text-lg leading-none">Sentinel</span>
                        <span className="text-xs text-muted-foreground leading-none mt-0.5">by CyberPath</span>
                    </div>
                </a>

                {/* Desktop Navigation */}
                <nav className="hidden md:flex items-center gap-8">
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
                        <Github className="h-4 w-4" />
                        GitHub
                    </a>
                </nav>

                {/* Desktop CTA */}
                <div className="hidden md:flex items-center gap-4">
                    <Button asChild variant="default" size="sm" className="gap-2">
                        <a href="/docs/introduction">
                            Get Started
                            <ExternalLink className="h-3.5 w-3.5" />
                        </a>
                    </Button>
                </div>

                {/* Mobile Menu */}
                <Sheet open={isOpen} onOpenChange={setIsOpen}>
                    <SheetTrigger asChild className="md:hidden">
                        <Button variant="ghost" size="icon">
                            <Menu className="h-5 w-5" />
                            <span className="sr-only">Toggle menu</span>
                        </Button>
                    </SheetTrigger>
                    <SheetContent side="right" className="w-72 bg-sidebar border-l border-border">
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
                                    <Github className="h-4 w-4" />
                                    GitHub
                                </a>
                            </nav>
                            <div className="pt-4 border-t border-border">
                                <Button asChild className="w-full gap-2">
                                    <a href="/docs/introduction">
                                        Get Started
                                        <ExternalLink className="h-3.5 w-3.5" />
                                    </a>
                                </Button>
                            </div>
                        </div>
                    </SheetContent>
                </Sheet>
            </div>
        </header>
    );
};

export default SiteHeader;
