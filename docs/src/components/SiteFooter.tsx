/**
 * Footer Component for Sentinel Documentation
 *
 * Modern footer with links, social media, and copyright
 */

import type { FC } from "react";
import { ExternalLink, Heart } from "lucide-react";

export const SiteFooter: FC = () => {
    const currentYear = new Date().getFullYear();

    const footerLinks = {
        documentation: [
            { href: "/docs/introduction", label: "Introduction" },
            { href: "/docs/installation", label: "Installation" },
            { href: "/docs/quick-start", label: "Quick Start" },
            { href: "/docs/store", label: "Store API" },
        ],
        resources: [
            {
                href: "https://github.com/cyberpath-HQ/sentinel",
                label: "GitHub Repository",
                external: true,
            },
            {
                href: "https://github.com/cyberpath-HQ/sentinel/issues",
                label: "Issue Tracker",
                external: true,
            },
            {
                href: "https://github.com/cyberpath-HQ/sentinel/blob/main/CHANGELOG.md",
                label: "Changelog",
                external: true,
            },
        ],
        community: [
            {
                href: "https://discord.gg/WmPc56hYut",
                label: "Discord Server",
                external: true,
            },
            {
                href: "https://x.com/cyberpath_hq",
                label: "Twitter/X",
                external: true,
            },
            {
                href: "https://cyberpath-hq.com",
                label: "CyberPath HQ",
                external: true,
            },
        ],
    };

    return (
        <footer className="border-t border-border bg-card/30">
            <div className="container mx-auto px-4 lg:px-8 py-12">
                <div className="grid gap-8 md:grid-cols-2 lg:grid-cols-4">
                    {/* Brand */}
                    <div className="lg:col-span-1 space-y-4">
                        <div className="flex items-center gap-3 h-9">
                            <img src="/logo-white.svg" alt="Sentinel Logo" className="h-8 w-auto" />
                        </div>
                        <p className="text-sm text-muted-foreground leading-relaxed">
                            A filesystem-backed document DBMS designed for organizations that prioritize trust,
                            transparency, and compliance over raw throughput.
                        </p>
                    </div>

                    {/* Documentation Links */}
                    <div className="space-y-4">
                        <h3 className="text-sm font-semibold uppercase tracking-wider text-foreground">Documentation</h3>
                        <ul className="space-y-3">
                            {footerLinks.documentation.map((link) => (
                                <li key={link.href}>
                                    <a
                                        href={link.href}
                                        className="text-sm text-muted-foreground hover:text-primary transition-colors"
                                    >
                                        {link.label}
                                    </a>
                                </li>
                            ))}
                        </ul>
                    </div>

                    {/* Resources Links */}
                    <div className="space-y-4">
                        <h3 className="text-sm font-semibold uppercase tracking-wider text-foreground">Resources</h3>
                        <ul className="space-y-3">
                            {footerLinks.resources.map((link) => (
                                <li key={link.href}>
                                    <a
                                        href={link.href}
                                        target={link.external ? "_blank" : undefined}
                                        rel={link.external ? "noopener noreferrer" : undefined}
                                        className="text-sm text-muted-foreground hover:text-primary transition-colors inline-flex items-center gap-1"
                                    >
                                        {link.label}
                                        {link.external && <ExternalLink className="h-3 w-3" />}
                                    </a>
                                </li>
                            ))}
                        </ul>
                    </div>

                    {/* Community Links */}
                    <div className="space-y-4">
                        <h3 className="text-sm font-semibold uppercase tracking-wider text-foreground">Community</h3>
                        <ul className="space-y-3">
                            {footerLinks.community.map((link) => (
                                <li key={link.href}>
                                    <a
                                        href={link.href}
                                        target={link.external ? "_blank" : undefined}
                                        rel={link.external ? "noopener noreferrer" : undefined}
                                        className="text-sm text-muted-foreground hover:text-primary transition-colors inline-flex items-center gap-1"
                                    >
                                        {link.label}
                                        {link.external && <ExternalLink className="h-3 w-3" />}
                                    </a>
                                </li>
                            ))}
                        </ul>
                    </div>
                </div>

                {/* Bottom Bar */}
                <div className="mt-12 pt-8 border-t border-border flex flex-col sm:flex-row items-center justify-between gap-4">
                    <div className="text-sm text-muted-foreground">
                        © {currentYear} CyberPath. Released under the Apache 2.0 License.
                    </div>
                    <div className="flex items-center gap-1 text-sm text-muted-foreground">
                        <span>Made with</span>
                        <Heart className="h-4 w-4 text-primary fill-primary" />
                        <span>by the CyberPath team</span>
                    </div>
                </div>
            </div>
        </footer>
    );
};

export default SiteFooter;
