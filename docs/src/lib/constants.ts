export const ALL_SEVERITIES = [
    `none`,
    `low`,
    `medium`,
    `high`,
    `critical`,
] as const;

export const SEVERITY_COLOR_MAP: Record<string, string> = {
    none:     `#87CEEB`,
    low:      `#32CD32`,
    medium:   `#FFD700`,
    high:     `#FF6347`,
    critical: `#8A2BE2`,
};

// Constants
export const ZERO = 0;
export const UNIT = 1;
export const TRANSPARENCY_MIN = 0;
export const TRANSPARENCY_MAX = 100;
export const BAR_RADIUS_MIN = 0;
export const BAR_RADIUS_MAX = 20;
export const INNER_RADIUS_MIN = 0;
export const INNER_RADIUS_MAX = 80;
export const DEFAULT_INNER_RADIUS = 60;
export const DEFAULT_TRANSPARENCY = 70;
export const DEFAULT_BAR_RADIUS = 5;
export const RGB_RED_START = 0;
export const RGB_RED_END = 2;
export const RGB_GREEN_START = RGB_RED_END;
export const RGB_GREEN_END = 4;
export const RGB_BLUE_START = RGB_GREEN_END;
export const RGB_BLUE_END = 6;
export const HUNDRED = 100;
export const THROTTLE_DELAY = 100;
export const DEFAULT_FRACTION_DIGITS = 2;
export const IMPACT_FRACTION_DIGITS = 1;
export const BAR_CHART_DEFAULT_LEFT_MARGIN = 20;
export const BAR_CHART_DEFAULT_BOTTOM_MARGIN = 20;
export const BAR_CHART_FALLBACK_BOTTOM_MARGIN = 5;
export const BAR_CHART_X_AXIS_DEFAULT_OFFSET = 5;
export const BAR_CHART_X_AXIS_FALLBACK_OFFSET = -5;
export const BAR_CHART_Y_AXIS_FALLBACK_WIDTH = 25;

export const CHART_DIALOG_SETTINGS_KEY = `chart-dialog-settings`;
export const CHART_NOT_FOUND_MESSAGE = `Chart not found`;
export const EXPORT_FAILED_MESSAGE = `Failed to export chart`;
export const EXPORT_SUCCESS_MESSAGE = `Chart exported as PNG`;
export const EMBED_CODE_COPIED_MESSAGE = `Embeddable code copied`;

export const UTM_PARAMS = {
    utm_source:   `quant.cyberpath-hq.com`,
    utm_medium:   `referral`,
    utm_campaign: `website`,
} as const;
export const CHART_FILENAME_PREFIX = `cvss-chart-`;
export const CHART_FILENAME_SUFFIX = `.png`;
export const DARK_THEME_BGCOLOR = `#18181B`;
export const LIGHT_THEME_BGCOLOR = `#ffffff`;
export const EXPORT_SCALE = 2;
export const EXPORT_QUALITY = 1;
export const EMBED_IFRAME_WIDTH = `400`;
export const EMBED_IFRAME_HEIGHT = `600`;
export const EMBED_IFRAME_STYLE = `border:none;`;
export const SHOULD_COPY_DEFAULT_STYLES = false;

// Chart Dialog Constants
export const CHART_TYPE_BAR = `bar` as const;
export const CHART_TYPE_DONUT = `donut` as const;
export const LEGEND_POSITION_BELOW_TITLE = `below-title` as const;
export const LEGEND_POSITION_BELOW_CHART = `below-chart` as const;
export const TOOLTIP_CONTENT_TYPE_COUNT = `count` as const;
export const TOOLTIP_CONTENT_TYPE_PERCENTAGE = `percentage` as const;

// Chart Dialog UI Constants
export const ACCORDION_VALUE_CHART_SETTINGS = `chart-settings`;
export const ACCORDION_VALUE_TYPE_SPECIFIC_SETTINGS = `type-specific-settings`;
export const ACCORDION_VALUE_CUSTOMIZATION = `customization`;

// Chart Settings Default Labels
export const CHART_SETTINGS_TITLE_DEFAULT = `CVSS Scores Chart`;
export const CHART_SETTINGS_X_AXIS_LABEL_DEFAULT = `Severity`;
export const CHART_SETTINGS_Y_AXIS_LABEL_DEFAULT = `Count`;
export const CHART_SETTINGS_TOOLTIP_LABEL_DEFAULT = `Count`;

// UI Text and Messages
export const CHART_CONFIGURATION_TITLE = `Chart Configuration`;
export const CHART_CONFIGURATION_DESCRIPTION = `Customize and view charts for selected CVSS score entries.`;
export const CHART_SETTINGS_DESCRIPTION = `Configure the appearance and data of the chart.`;
export const BAR_CHART_SETTINGS_DESCRIPTION = `Customize the appearance of the bar chart.`;
export const BAR_CHART_SETTINGS_TITLE = `Bar Chart Settings`;
export const DONUT_CHART_SETTINGS_DESCRIPTION = `Customize the appearance of the donut chart.`;
export const DONUT_CHART_SETTINGS_TITLE = `Donut Chart Settings`;
export const CUSTOMIZATION_DESCRIPTION = `Personalize chart colors and severity labels.`;

// Chart Rendering Constants
export const CHART_HEIGHT = `h-96`;
export const DONUT_CHART_OUTER_RADIUS = 120;
export const DONUT_CHART_DEFAULT_COLOR = `#8884d8`;
export const BAR_CHART_DEFAULT_COLOR = `#8884d8`;
export const CARTESIAN_GRID_STROKE_DASHARRAY = `3 3`;
export const CHART_AXIS_POSITION_BOTTOM = `bottom`;
export const CHART_AXIS_POSITION_INSIDE_LEFT = `insideLeft`;
export const CHART_AXIS_POSITION_ANGLE = -90;
export const PIE_CHART_POSITION_PERCENT = `50%`;
export const LOGO_ALT_TEXT = `CyberPath Quant Logo`;

// DOM Constants
export const DOM_ERROR_LOG_MESSAGE = `Export failed:`;
export const PARSE_ERROR_LOG_MESSAGE = `Failed to parse chart settings`;
export const IMAGE_LOADING_LAZY = `lazy`;

// Logo Styling
export const LOGO_POSITION_TOP = `top-1.25`;
export const LOGO_POSITION_RIGHT = `right-4`;
export const LOGO_HEIGHT = `h-5`;
export const LOGO_WIDTH = `w-auto`;

// Responsive Container Dimensions
export const RESPONSIVE_CONTAINER_WIDTH = `100%`;
export const RESPONSIVE_CONTAINER_HEIGHT = `100%`;

// Default Settings
export const DEFAULT_SETTINGS = {
    chart_type:                     CHART_TYPE_BAR as `bar` | `donut`,
    title:                          CHART_SETTINGS_TITLE_DEFAULT,
    should_show_legend:             true,
    should_show_x_axis_label:       true,
    should_show_y_axis_label:       true,
    inner_radius:                   DEFAULT_INNER_RADIUS,
    custom_colors:                  {} as Record<string, string>,
    transparency:                   DEFAULT_TRANSPARENCY,
    x_axis_label:                   CHART_SETTINGS_X_AXIS_LABEL_DEFAULT,
    y_axis_label:                   CHART_SETTINGS_Y_AXIS_LABEL_DEFAULT,
    tooltip_label:                  CHART_SETTINGS_TOOLTIP_LABEL_DEFAULT,
    bar_radius:                     DEFAULT_BAR_RADIUS,
    severity_labels:                {} as Record<string, string>,
    should_show_floating_labels:    true,
    tooltip_content_type:           TOOLTIP_CONTENT_TYPE_COUNT as `count` | `percentage`,
    floating_label_type:            TOOLTIP_CONTENT_TYPE_PERCENTAGE as `count` | `percentage`,
    should_show_x_axis_tick_labels: true,
    legend_position:                LEGEND_POSITION_BELOW_CHART as `below-chart` | `below-title`,
};

// CVSS Calculator Constants
export const CVSS_VERSION_2_0 = `2.0` as const;
export const CVSS_VERSION_3_0 = `3.0` as const;
export const CVSS_VERSION_3_1 = `3.1` as const;
export const CVSS_VERSION_4_0 = `4.0` as const;

export const CVSS_DEFAULT_ACTIVE_GROUP = `base-metrics`;
export const CVSS_METRICS_TAB_REPLACE_TEXT = `Metrics`;
export const CVSS_METRICS_TAB_TEMPORAL_TEXT = `Temporal`;
export const CVSS_METRICS_TAB_THREAT_TEXT = `Threat`;
export const CVSS_VECTOR_COPIED_MESSAGE = `Vector string copied`;
export const CVSS_SHAREABLE_LINK_MESSAGE = `Shareable link copied`;
export const CVSS_EMBEDDABLE_CODE_MESSAGE = `Embeddable code copied`;
export const CVSS_SCORE_SAVED_MESSAGE = `Score saved to history`;
export const CVSS_CONFIGURE_METRICS_TITLE = `Configure Metrics`;
export const CVSS_SCORE_UPDATES_SUBTITLE = `Score updates in real-time`;
export const CVSS_HISTORY_NAME_PLACEHOLDER = `Enter a name for this score`;
export const CVSS_ALTERNATIVE_DESCRIPTIONS_ID = `alternative-descriptions`;
export const CVSS_SHOW_CONTRIBUTIONS_ID = `show-contributions`;
export const CVSS_GRID_COLS_4 = 4;
export const CVSS_GRID_COLS_3 = 3;
export const CVSS_EMBED_PATH_40 = `cvss40`;
export const CVSS_EMBED_PATH_2 = `cvss2`;
export const CVSS_EMBED_PATH_3_PREFIX = `cvss3`;
export const CVSS_EMBED_IFRAME_WIDTH = `400`;
export const CVSS_EMBED_IFRAME_HEIGHT = `600`;
export const CVSS_EMBED_IFRAME_STYLE = `border:none;`;
export const CVSS_QUERY_PARAM_VECTOR = `vector`;
export const CVSS_LOGO_ALT = `CyberPath Quant Logo`;
export const CVSS_ENTER_KEY = `Enter`;
export const CVSS_THEME_LIGHT_ATTR = `data-light-theme`;
export const CVSS_THEME_DARK_ATTR = `data-dark-theme`;
export const CVSS_IMAGE_LOADING = `lazy`;
export const CVSS_SEVERITY_RATING_PREFIX = `CVSS v`;
