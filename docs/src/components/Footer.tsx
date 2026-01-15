/**
 * Footer Component
 *
 * Application footer with links and information
 */

import { UTM_PARAMS } from "@/lib/constants";

export function Footer() {
    const currentYear = new Date().getFullYear();

    return (
        <footer className="border-t bg-background text-foreground/90">
            <div className="container mx-auto px-4 py-8">
                <div className="grid gap-8 md:grid-cols-4">
                    <div className="space-y-3">
                        <h3 className="text-sm font-semibold">About Quant</h3>
                        <p className="text-sm text-muted-foreground">
                            An intuitive, modern CVSS score calculator for vulnerability severity assessment.
                        </p>
                    </div>
                    <div className="space-y-3">
                        <h3 className="text-sm font-semibold">Resources</h3>
                        <ul className="space-y-2 text-sm">
                            <li>
                                <a
                                    href={
                                        `https://www.first.org/cvss/v4-0/specification-document?${new URLSearchParams(UTM_PARAMS).toString()}`
                                    }
                                    target="_blank"
                                    rel="noopener noreferrer"
                                    className="text-muted-foreground hover:text-primary transition-colors"
                                >
                                    CVSS v4.0 Specification
                                </a>
                            </li>
                            <li>
                                <a
                                    href={
                                        `https://www.first.org/cvss/v3-1/specification-document?${new URLSearchParams(UTM_PARAMS).toString()}`
                                    }
                                    target="_blank"
                                    rel="noopener noreferrer"
                                    className="text-muted-foreground hover:text-primary transition-colors"
                                >
                                    CVSS v3.1 Specification
                                </a>
                            </li>
                            <li>
                                <a
                                    href={
                                        `https://www.first.org/cvss/v2/guide?${new URLSearchParams(UTM_PARAMS).toString()}`
                                    }
                                    target="_blank"
                                    rel="noopener noreferrer"
                                    className="text-muted-foreground hover:text-primary transition-colors"
                                >
                                    CVSS v2.0 Guide
                                </a>
                            </li>
                        </ul>
                    </div>
                    <div className="space-y-3">
                        <h3 className="text-sm font-semibold">Community</h3>
                        <ul className="space-y-2 text-sm">
                            <li>
                                <a
                                    href={
                                        `https://github.com/cyberpath-HQ/Quant?${new URLSearchParams(UTM_PARAMS).toString()}`
                                    }
                                    target="_blank"
                                    rel="noopener noreferrer"
                                    className="text-muted-foreground hover:text-primary transition-colors"
                                >
                                    GitHub Repository
                                </a>
                            </li>
                            <li>
                                <a
                                    href={
                                        `https://discord.gg/WmPc56hYut?${new URLSearchParams(UTM_PARAMS).toString()}`
                                    }
                                    target="_blank"
                                    rel="noopener noreferrer"
                                    className="text-muted-foreground hover:text-primary transition-colors"
                                >
                                    Discord Server
                                </a>
                            </li>
                            <li>
                                <a
                                    href={
                                        `https://x.com/cyberpath_hq?${new URLSearchParams(UTM_PARAMS).toString()}`
                                    }
                                    target="_blank"
                                    rel="noopener noreferrer"
                                    className="text-muted-foreground hover:text-primary transition-colors"
                                >
                                    Twitter/X
                                </a>
                            </li>
                        </ul>
                    </div>
                    <div className="space-y-3">
                        <h3 className="text-sm font-semibold">Legal</h3>
                        <ul className="space-y-2 text-sm">
                            <li>
                                <a
                                    href={
                                        `https://github.com/cyberpath-HQ/Quant/blob/main/LICENSE?${new URLSearchParams(UTM_PARAMS).toString()}`
                                    }
                                    target="_blank"
                                    rel="noopener noreferrer"
                                    className="text-muted-foreground hover:text-primary transition-colors"
                                >
                                    Apache 2.0 License
                                </a>
                            </li>
                            <li>
                                <a
                                    href={
                                        `https://cyberpath-hq.com?${new URLSearchParams(UTM_PARAMS).toString()}`
                                    }
                                    target="_blank"
                                    rel="noopener"
                                    className="text-muted-foreground hover:text-primary transition-colors"
                                >
                                    Cyberpath
                                </a>
                            </li>
                        </ul>
                    </div>
                </div>
                <div className="mt-8 border-t pt-8 text-center text-sm text-muted-foreground">
                    <p>© {currentYear} Cyberpath. Built with ❤️ to make vulnerability assessment accessible to everyone.</p>
                </div>
            </div>
        </footer>
    );
}
