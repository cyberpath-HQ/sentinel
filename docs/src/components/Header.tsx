/**
 * Header Component
 *
 * Application header with logo, navigation, and theme toggle
 */

import {
    useState, useEffect,
    type FC
} from "react";
import { Button } from "@/components/ui/button";
import {
    Sheet,
    SheetContent,
    SheetTrigger
} from "@/components/ui/sheet";
import { Separator } from "@/components/ui/separator";
import {
    Moon, Sun, Menu
} from "lucide-react";
import logoBlack from "@/assets/logo.svg";
import logoWhite from "@/assets/logo-white.svg";
import { cn } from "@/lib/utils";
import { UTM_PARAMS } from "@/lib/constants";

interface HeaderProps {
    activePage?: `home` | `calculator` | `documentation`
}

export const Header: FC<HeaderProps> = ({
    activePage = `home`,
}) => {
    const [
        theme,
        setTheme,
    ] = useState<`light` | `dark`>(`light`);

    const [
        isOpen,
        setIsOpen,
    ] = useState(false);

    useEffect(() => {
        // Check for saved theme preference or default to light mode
        const savedTheme = localStorage.getItem(`theme`) as `light` | `dark` | null;
        const prefersDark = window.matchMedia(`(prefers-color-scheme: dark)`).matches;
        const initialTheme = savedTheme || (prefersDark ? `dark` : `light`);
        setTheme(initialTheme);
        document.documentElement.classList.toggle(`dark`, initialTheme === `dark`);
    }, []);

    const toggleTheme = () => {
        const newTheme = theme === `light` ? `dark` : `light`;
        setTheme(newTheme);
        localStorage.setItem(`theme`, newTheme);
        document.documentElement.classList.toggle(`dark`, newTheme === `dark`);
    };

    return (
        <header className="sticky top-0 z-50 w-full border-b bg-background/95 backdrop-blur">
            <div className="container mx-auto flex h-16 items-center justify-between px-4">
                <div className="flex items-center gap-6">
                    <a href="/" className="flex items-center gap-2">
                        <picture className="block dark:hidden" data-light-theme>
                            <img
                                src={logoBlack.src}
                                loading="lazy"
                                alt="CyberPath Quant Logo"
                                className="h-8 w-auto dark:hidden"
                            />
                        </picture>

                        {/* Dark theme image */}
                        <picture className="hidden dark:block" data-dark-theme>
                            <img
                                src={logoWhite.src}
                                loading="lazy"
                                alt="CyberPath Quant Logo"
                                className="h-8 w-auto hidden dark:block"
                            />
                        </picture>
                    </a>
                    <nav className="hidden md:flex gap-6">
                        <a href="/"
                            className={cn(
                                `text-sm font-medium transition-colors hover:text-primary`,
                                activePage === `home` ? `text-foreground` : `text-muted-foreground`
                            )}>
                            Home
                        </a>
                        <a href="/calculator"
                            className={cn(
                                `text-sm font-medium transition-colors hover:text-primary`,
                                activePage === `calculator` ? `text-foreground` : `text-muted-foreground`
                            )}>
                            Calculator
                        </a>
                        <a href="/docs"
                            className={cn(
                                `text-sm font-medium transition-colors hover:text-primary`,
                                activePage === `documentation` ? `text-foreground` : `text-muted-foreground`
                            )}>
                            Documentation
                        </a>
                        <a
                            href={
                                `https://github.com/cyberpath-HQ/Quant?${ new URLSearchParams(UTM_PARAMS).toString() }`
                            }
                            target="_blank"
                            rel="noopener"
                            className="text-sm font-medium text-muted-foreground transition-colors hover:text-primary"
                        >
                            GitHub
                        </a>
                    </nav>
                </div>
                <div className="flex items-center gap-2">
                    <Button variant="ghost" size="icon" onClick={toggleTheme} aria-label="Toggle theme" className="hidden md:inline-flex">
                        {theme === `light` ? <Moon className="h-5 w-5" /> : <Sun className="h-5 w-5" />}
                    </Button>
                    <Sheet open={isOpen} onOpenChange={setIsOpen}>
                        <SheetTrigger asChild>
                            <Button variant="ghost" size="icon" className="md:hidden">
                                <Menu className="h-5 w-5" />
                            </Button>
                        </SheetTrigger>
                        <SheetContent side="right">
                            <div className="space-y-4 mt-6">
                                <Button
                                    variant="ghost"
                                    className={cn(
                                        `w-full justify-start font-medium`,
                                        activePage === `home` ? `text-foreground` : `text-muted-foreground`
                                    )}
                                    asChild>
                                    <a href="/" onClick={() => setIsOpen(false)}>
                                        Home
                                    </a>
                                </Button>
                                <Button
                                    variant="ghost"
                                    className={cn(
                                        `w-full justify-start font-medium`,
                                        activePage === `calculator` ? `text-foreground` : `text-muted-foreground`
                                    )}
                                    asChild>
                                    <a href="/calculator" onClick={() => setIsOpen(false)}>
                                        Calculator
                                    </a>
                                </Button>
                                <Button
                                    variant="ghost"
                                    className={cn(
                                        `w-full justify-start font-medium`,
                                        activePage === `documentation` ? `text-foreground` : `text-muted-foreground`
                                    )}
                                    asChild>
                                    <a href="/docs" onClick={() => setIsOpen(false)}>
                                        Documentation
                                    </a>
                                </Button>
                                <Button variant="ghost" className="w-full justify-start font-medium text-muted-foreground" asChild>
                                    <a
                                        href={
                                            `https://github.com/cyberpath-HQ/Quant?${ new URLSearchParams(UTM_PARAMS).toString() }`
                                        }
                                        target="_blank"
                                        rel="noopener noreferrer"
                                        onClick={() => setIsOpen(false)}
                                    >
                                        GitHub
                                    </a>
                                </Button>
                                <Separator />
                                <Button variant="ghost" className="w-full justify-start" onClick={() => {
                                    toggleTheme();
                                    setIsOpen(false);
                                }}>
                                    {theme === `light` ? <Moon className="h-4 w-4 mr-2" /> : <Sun className="h-4 w-4 mr-2" />}
                                    Toggle Theme
                                </Button>
                            </div>
                        </SheetContent>
                    </Sheet>
                </div>
            </div>
        </header>
    );
};
