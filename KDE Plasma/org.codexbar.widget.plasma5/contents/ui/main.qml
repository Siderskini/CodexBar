import QtQuick 2.15
import QtQuick.Layouts 1.15
import org.kde.kirigami 2.20 as Kirigami
import org.kde.plasma.core 2.0 as PlasmaCore
import org.kde.plasma.plasmoid 2.0

Item {
    id: root

    property var snapshot: ({generatedAt: "", entries: []})
    property string lastError: ""
    property bool isShuttingDown: false
    property string selectedProvider: ""
    property bool useDarkTheme: root.isDarkThemeColor(Kirigami.Theme.backgroundColor)
    property real cardOpacity: 0.96

    readonly property int fallbackIconSize: Math.round(Kirigami.Units.gridUnit * 1.9)
    readonly property int panelIconSizeRaw: (typeof Plasmoid.iconSize === "number" && isFinite(Plasmoid.iconSize))
        ? Math.round(Plasmoid.iconSize)
        : fallbackIconSize
    readonly property int iconSizeLowerBound: Math.max(14, Math.round(Kirigami.Units.gridUnit * 1.4))
    readonly property int iconSizeUpperBound: Math.max(iconSizeLowerBound, Math.round(Kirigami.Units.gridUnit * 2.8))
    property int baseIconSize: Math.max(iconSizeLowerBound, Math.min(iconSizeUpperBound,
        panelIconSizeRaw > 0 ? panelIconSizeRaw : fallbackIconSize))
    property int iconWidth: Plasmoid.formFactor === PlasmaCore.Types.Vertical ? baseIconSize : Math.round(baseIconSize * 2.1)
    property int iconHeight: baseIconSize

    property color cardBackground: useDarkTheme ? "#252A3A" : "#D6D2F0"
    property color cardBorder: useDarkTheme ? "#4D5576" : "#B7B1DA"
    property color sectionDivider: useDarkTheme ? "#586083" : "#B9B3DD"
    property color trackColor: useDarkTheme ? "#3E4664" : "#C6C1E9"
    property color tabActive: "#3B78DE"
    property color tabInactive: useDarkTheme ? "#353D57" : "#CAC6EA"
    property color textStrong: useDarkTheme ? "#F1F3FF" : "#2F2C45"
    property color textMuted: useDarkTheme ? "#C0C6E4" : "#6F6B8E"
    property color badgeBackground: useDarkTheme ? "#424A69" : Qt.rgba(1, 1, 1, 0.35)

    Plasmoid.backgroundHints: PlasmaCore.Types.NoBackground
    Plasmoid.preferredRepresentation: Plasmoid.compactRepresentation
    Plasmoid.toolTipMainText: i18n("CodexBar")
    Plasmoid.toolTipSubText: lastError.length > 0
        ? lastError
        : root.entrySummary(root.currentEntry())

    onSnapshotChanged: {
        if (selectedProvider.length > 0) {
            return;
        }

        var codexEntry = entryForProvider("codex");
        if (codexEntry) {
            selectedProvider = "codex";
            return;
        }

        selectedProvider = (snapshot.entries && snapshot.entries.length > 0)
            ? normalizedProvider(snapshot.entries[0].provider)
            : "codex";
    }

    function entryForProvider(providerId) {
        if (!snapshot.entries || snapshot.entries.length === 0) {
            return null;
        }

        var requested = normalizedProvider(providerId);
        for (var i = 0; i < snapshot.entries.length; ++i) {
            var entry = snapshot.entries[i];
            if (entry && normalizedProvider(entry.provider) === requested) {
                return entry;
            }
        }

        return null;
    }

    function isDarkThemeColor(colorValue) {
        var r = Number(colorValue.r);
        var g = Number(colorValue.g);
        var b = Number(colorValue.b);
        if (!isFinite(r) || !isFinite(g) || !isFinite(b)) {
            return false;
        }

        var luminance = (0.2126 * r) + (0.7152 * g) + (0.0722 * b);
        return luminance < 0.52;
    }

    function applyAlpha(baseColor, alphaValue) {
        var alpha = Math.max(0, Math.min(1, Number(alphaValue)));
        return Qt.rgba(baseColor.r, baseColor.g, baseColor.b, alpha);
    }

    function currentEntry() {
        var selected = entryForProvider(selectedProvider);
        if (selected) {
            return selected;
        }

        if (!snapshot.entries || snapshot.entries.length === 0) {
            return null;
        }

        return snapshot.entries[0];
    }

    function clampPercent(value) {
        var number = Number(value);
        if (isNaN(number) || !isFinite(number)) {
            return 0;
        }
        return Math.max(0, Math.min(100, number));
    }

    function hasUsage(windowData) {
        return !!windowData
            && windowData.usedPercent !== undefined
            && windowData.usedPercent !== null
            && !isNaN(Number(windowData.usedPercent));
    }

    function usedPercent(windowData) {
        if (!hasUsage(windowData)) {
            return 0;
        }
        return clampPercent(windowData.usedPercent);
    }

    function remainingPercent(windowData) {
        return clampPercent(100 - usedPercent(windowData));
    }

    function usageColor(windowData) {
        var used = usedPercent(windowData);
        if (useDarkTheme) {
            if (used >= 90) {
                return "#E36A6A";
            }
            if (used >= 70) {
                return "#E39A61";
            }
            return "#E5B174";
        }

        if (used >= 90) {
            return "#C84A4A";
        }
        if (used >= 70) {
            return "#D06C38";
        }
        return "#D28A52";
    }

    function windowLabel(windowData, fallbackLabel) {
        if (!windowData || windowData.windowMinutes === undefined || windowData.windowMinutes === null) {
            return fallbackLabel;
        }

        var minutes = Number(windowData.windowMinutes);
        if (!isFinite(minutes) || minutes <= 0) {
            return fallbackLabel;
        }
        if (minutes === 300) {
            return "5h";
        }
        if (minutes === 10080) {
            return "7d";
        }
        if (minutes % 1440 === 0) {
            return Math.round(minutes / 1440) + "d";
        }
        if (minutes % 60 === 0) {
            return Math.round(minutes / 60) + "h";
        }
        return Math.round(minutes) + "m";
    }

    function parseTimestampMs(rawTimestamp) {
        if (!rawTimestamp) {
            return NaN;
        }

        var raw = String(rawTimestamp);
        if (raw.indexOf("unix:") === 0) {
            var seconds = Number(raw.slice(5));
            if (!isNaN(seconds) && isFinite(seconds)) {
                return seconds * 1000;
            }
            return NaN;
        }

        var parsed = Date.parse(raw);
        if (!isNaN(parsed)) {
            return parsed;
        }

        return NaN;
    }

    function durationLabelFromMs(milliseconds) {
        var totalMinutes = Math.max(0, Math.round(milliseconds / 60000));
        if (totalMinutes <= 0) {
            return "<1m";
        }

        var days = Math.floor(totalMinutes / 1440);
        var hours = Math.floor((totalMinutes % 1440) / 60);
        var minutes = totalMinutes % 60;

        if (days > 0) {
            return days + "d " + hours + "h";
        }
        if (hours > 0) {
            return hours + "h " + minutes + "m";
        }
        return minutes + "m";
    }

    function formatRelative(rawTimestamp) {
        var timestampMs = parseTimestampMs(rawTimestamp);
        if (isNaN(timestampMs)) {
            return "unknown";
        }

        var nowMs = Date.now();
        var deltaMs = nowMs - timestampMs;
        var absMs = Math.abs(deltaMs);

        if (absMs < 45000) {
            return "just now";
        }

        if (absMs < 3600000) {
            var minutes = Math.max(1, Math.round(absMs / 60000));
            return deltaMs >= 0 ? (minutes + "m ago") : ("in " + minutes + "m");
        }

        if (absMs < 86400000) {
            var hours = Math.max(1, Math.round(absMs / 3600000));
            return deltaMs >= 0 ? (hours + "h ago") : ("in " + hours + "h");
        }

        var days = Math.max(1, Math.round(absMs / 86400000));
        return deltaMs >= 0 ? (days + "d ago") : ("in " + days + "d");
    }

    function resetCountdown(windowData) {
        if (!windowData || !windowData.resetsAt) {
            return "Resets unknown";
        }

        var resetMs = parseTimestampMs(windowData.resetsAt);
        if (isNaN(resetMs)) {
            return "Resets unknown";
        }

        var deltaMs = resetMs - Date.now();
        if (deltaMs <= 0) {
            return "Resetting soon";
        }

        return "Resets in " + durationLabelFromMs(deltaMs);
    }

    function sourceLabel(source) {
        if (!source) {
            return "unknown";
        }

        var raw = String(source);
        var lower = raw.toLowerCase();
        if (lower === "codex-cli") {
            return "codex RPC";
        }
        if (lower === "codex-status") {
            return "codex /status";
        }
        if (lower === "claude-cli") {
            return "claude /usage";
        }
        if (lower === "openai-web") {
            return "OpenAI web";
        }
        if (lower === "oauth") {
            return "OAuth";
        }
        return raw;
    }

    function usageText(windowData) {
        if (!hasUsage(windowData)) {
            return "n/a";
        }
        return Math.round(usedPercent(windowData)) + "% used";
    }

    function updatedLabel(entry) {
        if (!entry) {
            return "Updated unknown";
        }
        return "Updated " + formatRelative(entry.updatedAt);
    }

    function codeReviewUsedPercent(entry) {
        if (!entry || entry.codeReviewRemainingPercent === undefined || entry.codeReviewRemainingPercent === null) {
            return -1;
        }

        var remaining = Number(entry.codeReviewRemainingPercent);
        if (isNaN(remaining) || !isFinite(remaining)) {
            return -1;
        }

        return clampPercent(100 - remaining);
    }

    function creditsText(entry) {
        if (!entry || entry.creditsRemaining === undefined || entry.creditsRemaining === null) {
            return "No credit data available";
        }
        return "Credits remaining: " + Number(entry.creditsRemaining).toFixed(1);
    }

    function codeReviewUsedText(entry) {
        var used = codeReviewUsedPercent(entry);
        if (used < 0) {
            return "";
        }
        return Math.round(used) + "% used";
    }

    function entrySummary(entry) {
        if (!entry) {
            return i18n("No live provider data yet");
        }

        var sessionPart = windowLabel(entry.primary, "Session") + ": " + usageText(entry.primary);
        var weeklyPart = windowLabel(entry.secondary, "Weekly") + ": " + usageText(entry.secondary);
        return entry.provider + " | " + sessionPart + " | " + weeklyPart + " | " + sourceLabel(entry.source);
    }

    function nonEmptyString(value) {
        if (value === undefined || value === null) {
            return "";
        }

        var text = String(value).trim();
        return text.length > 0 ? text : "";
    }

    function normalizedProvider(providerId) {
        var normalized = nonEmptyString(providerId).toLowerCase();
        return normalized.length > 0 ? normalized : "codex";
    }

    function isClaudeProvider(providerId) {
        return normalizedProvider(providerId) === "claude";
    }

    function emptyEntryForProvider(providerId) {
        return {
            provider: normalizedProvider(providerId),
            source: "",
            updatedAt: "",
            primary: null,
            secondary: null,
            tertiary: null,
            creditsRemaining: null,
            codeReviewRemainingPercent: null,
            status: null
        };
    }

    function displayEntry() {
        var provider = preferredProviderId();
        var entry = entryForProvider(provider);
        if (entry) {
            return entry;
        }

        return emptyEntryForProvider(provider);
    }

    function hasAnyUsageData(entry) {
        if (!entry) {
            return false;
        }

        return hasUsage(entry.primary)
            || hasUsage(entry.secondary)
            || hasUsage(entry.tertiary)
            || codeReviewUsedPercent(entry) >= 0
            || (entry.creditsRemaining !== undefined && entry.creditsRemaining !== null && isFinite(Number(entry.creditsRemaining)));
    }

    function tabLabelForProvider(providerId) {
        var provider = normalizedProvider(providerId);
        if (provider === "factory") {
            return "Droid";
        }

        return provider.charAt(0).toUpperCase() + provider.slice(1);
    }

    function tabIconColor(active) {
        if (active) {
            return "#FFFFFF";
        }
        return useDarkTheme ? "#DCE2FF" : "#4D4969";
    }

    function drawIconLine(ctx, x1, y1, x2, y2) {
        ctx.beginPath();
        ctx.moveTo(x1, y1);
        ctx.lineTo(x2, y2);
        ctx.stroke();
    }

    function drawIconRoundedRect(ctx, x, y, width, height, radius) {
        var r = Math.min(radius, width / 2, height / 2);
        ctx.beginPath();
        ctx.moveTo(x + r, y);
        ctx.lineTo(x + width - r, y);
        ctx.quadraticCurveTo(x + width, y, x + width, y + r);
        ctx.lineTo(x + width, y + height - r);
        ctx.quadraticCurveTo(x + width, y + height, x + width - r, y + height);
        ctx.lineTo(x + r, y + height);
        ctx.quadraticCurveTo(x, y + height, x, y + height - r);
        ctx.lineTo(x, y + r);
        ctx.quadraticCurveTo(x, y, x + r, y);
        ctx.closePath();
    }

    function paintProviderTabIcon(ctx, providerId, width, height, active) {
        var provider = normalizedProvider(providerId);
        var unit = Math.min(width, height);
        var centerX = width / 2;
        var centerY = height / 2;
        var color = tabIconColor(active);

        ctx.clearRect(0, 0, width, height);
        ctx.strokeStyle = color;
        ctx.fillStyle = color;
        ctx.lineCap = "round";
        ctx.lineJoin = "round";

        if (provider === "codex") {
            ctx.lineWidth = Math.max(1.1, unit * 0.10);
            for (var i = 0; i < 3; ++i) {
                var angle = (-Math.PI / 2) + (i * (Math.PI * 2 / 3));
                var ox = centerX + Math.cos(angle) * unit * 0.14;
                var oy = centerY + Math.sin(angle) * unit * 0.14;
                ctx.beginPath();
                ctx.arc(ox, oy, unit * 0.20, 0, Math.PI * 2, false);
                ctx.stroke();
            }
            return;
        }

        if (provider === "claude") {
            ctx.lineWidth = Math.max(1.0, unit * 0.085);
            for (var r = 0; r < 8; ++r) {
                var rayAngle = r * Math.PI / 4;
                drawIconLine(
                    ctx,
                    centerX + Math.cos(rayAngle) * unit * 0.12,
                    centerY + Math.sin(rayAngle) * unit * 0.12,
                    centerX + Math.cos(rayAngle) * unit * 0.35,
                    centerY + Math.sin(rayAngle) * unit * 0.35
                );
            }
            ctx.beginPath();
            ctx.arc(centerX, centerY, unit * 0.09, 0, Math.PI * 2, false);
            ctx.fill();
            return;
        }

        if (provider === "cursor") {
            var topY = centerY - unit * 0.28;
            var midY = centerY - unit * 0.04;
            var bottomY = centerY + unit * 0.24;
            var leftX = centerX - unit * 0.26;
            var rightX = centerX + unit * 0.26;
            ctx.lineWidth = Math.max(1.0, unit * 0.09);

            ctx.beginPath();
            ctx.moveTo(centerX, topY);
            ctx.lineTo(rightX, midY);
            ctx.lineTo(centerX, centerY + unit * 0.08);
            ctx.lineTo(leftX, midY);
            ctx.closePath();
            ctx.stroke();

            ctx.beginPath();
            ctx.moveTo(centerX, centerY + unit * 0.08);
            ctx.lineTo(rightX, bottomY);
            ctx.lineTo(centerX, centerY + unit * 0.34);
            ctx.lineTo(leftX, bottomY);
            ctx.closePath();
            ctx.stroke();

            drawIconLine(ctx, leftX, midY, leftX, bottomY);
            drawIconLine(ctx, rightX, midY, rightX, bottomY);
            return;
        }

        if (provider === "factory") {
            ctx.lineWidth = Math.max(1.0, unit * 0.09);
            for (var s = 0; s < 6; ++s) {
                var spokeAngle = s * Math.PI / 3;
                drawIconLine(
                    ctx,
                    centerX - Math.cos(spokeAngle) * unit * 0.32,
                    centerY - Math.sin(spokeAngle) * unit * 0.32,
                    centerX + Math.cos(spokeAngle) * unit * 0.32,
                    centerY + Math.sin(spokeAngle) * unit * 0.32
                );
            }
            ctx.beginPath();
            ctx.arc(centerX, centerY, unit * 0.07, 0, Math.PI * 2, false);
            ctx.fill();
            return;
        }

        if (provider === "gemini") {
            ctx.lineWidth = Math.max(1.0, unit * 0.09);
            ctx.beginPath();
            ctx.moveTo(centerX, centerY - unit * 0.34);
            ctx.lineTo(centerX + unit * 0.14, centerY - unit * 0.12);
            ctx.lineTo(centerX + unit * 0.34, centerY);
            ctx.lineTo(centerX + unit * 0.14, centerY + unit * 0.12);
            ctx.lineTo(centerX, centerY + unit * 0.34);
            ctx.lineTo(centerX - unit * 0.14, centerY + unit * 0.12);
            ctx.lineTo(centerX - unit * 0.34, centerY);
            ctx.lineTo(centerX - unit * 0.14, centerY - unit * 0.12);
            ctx.closePath();
            ctx.stroke();

            ctx.lineWidth = Math.max(0.9, unit * 0.07);
            drawIconLine(ctx, centerX + unit * 0.2, centerY - unit * 0.28, centerX + unit * 0.2, centerY - unit * 0.06);
            drawIconLine(ctx, centerX + unit * 0.09, centerY - unit * 0.17, centerX + unit * 0.31, centerY - unit * 0.17);
            return;
        }

        if (provider === "copilot") {
            ctx.lineWidth = Math.max(1.0, unit * 0.08);
            drawIconRoundedRect(
                ctx,
                centerX - unit * 0.34,
                centerY - unit * 0.24,
                unit * 0.68,
                unit * 0.50,
                unit * 0.14
            );
            ctx.stroke();

            ctx.beginPath();
            ctx.arc(centerX - unit * 0.13, centerY - unit * 0.02, unit * 0.055, 0, Math.PI * 2, false);
            ctx.fill();
            ctx.beginPath();
            ctx.arc(centerX + unit * 0.13, centerY - unit * 0.02, unit * 0.055, 0, Math.PI * 2, false);
            ctx.fill();

            ctx.beginPath();
            ctx.moveTo(centerX - unit * 0.16, centerY + unit * 0.14);
            ctx.quadraticCurveTo(centerX, centerY + unit * 0.22, centerX + unit * 0.16, centerY + unit * 0.14);
            ctx.stroke();
            return;
        }

        ctx.lineWidth = Math.max(1.0, unit * 0.09);
        ctx.beginPath();
        ctx.arc(centerX, centerY, unit * 0.28, 0, Math.PI * 2, false);
        ctx.stroke();
    }

    function weeklyUsageForProvider(providerId) {
        var entry = entryForProvider(providerId);
        return usedPercent(entry ? entry.secondary : null);
    }

    function weeklyUsageColorForProvider(providerId) {
        var entry = entryForProvider(providerId);
        return usageColor(entry ? entry.secondary : null);
    }

    function tabModel() {
        var preferredProviders = ["codex", "claude", "cursor", "factory", "gemini", "copilot"];
        var tabs = [];
        var seen = {};

        for (var i = 0; i < preferredProviders.length; ++i) {
            var preferred = normalizedProvider(preferredProviders[i]);
            seen[preferred] = true;
            tabs.push({provider: preferred, label: tabLabelForProvider(preferred)});
        }

        if (snapshot.entries && snapshot.entries.length > 0) {
            for (var j = 0; j < snapshot.entries.length; ++j) {
                var dynamicProvider = normalizedProvider(snapshot.entries[j].provider);
                if (seen[dynamicProvider]) {
                    continue;
                }
                seen[dynamicProvider] = true;
                tabs.push({provider: dynamicProvider, label: tabLabelForProvider(dynamicProvider)});
            }
        }

        var selectedRaw = nonEmptyString(selectedProvider);
        if (selectedRaw.length > 0) {
            var selected = normalizedProvider(selectedRaw);
            if (!seen[selected]) {
                tabs.push({provider: selected, label: tabLabelForProvider(selected)});
            }
        }

        return tabs;
    }

    function preferredProviderId() {
        var selected = normalizedProvider(selectedProvider);
        if (selectedProvider.length > 0) {
            return selected;
        }

        if (snapshot.entries && snapshot.entries.length > 0) {
            for (var i = 0; i < snapshot.entries.length; ++i) {
                var provider = normalizedProvider(snapshot.entries[i].provider);
                if (provider === "codex") {
                    return provider;
                }
            }

            return normalizedProvider(snapshot.entries[0].provider);
        }

        if (snapshot.enabledProviders && snapshot.enabledProviders.length > 0) {
            for (var j = 0; j < snapshot.enabledProviders.length; ++j) {
                var enabledProvider = normalizedProvider(snapshot.enabledProviders[j]);
                if (enabledProvider === "codex") {
                    return enabledProvider;
                }
            }

            return normalizedProvider(snapshot.enabledProviders[0]);
        }

        return "codex";
    }

    function preferredProviderEntry() {
        var provider = preferredProviderId();
        var entry = entryForProvider(provider);
        if (entry) {
            return entry;
        }

        return currentEntry();
    }

    function providerMetadata(providerId) {
        var metadataByProvider = {
            codex: {
                dashboardUrl: "https://chatgpt.com/codex/settings/usage",
                statusPageUrl: "https://status.openai.com/",
                statusLinkUrl: "",
                subscriptionDashboardUrl: ""
            },
            claude: {
                dashboardUrl: "https://console.anthropic.com/settings/billing",
                statusPageUrl: "https://status.claude.com/",
                statusLinkUrl: "",
                subscriptionDashboardUrl: "https://claude.ai/settings/usage"
            },
            cursor: {
                dashboardUrl: "https://cursor.com/dashboard?tab=usage",
                statusPageUrl: "https://status.cursor.com",
                statusLinkUrl: "",
                subscriptionDashboardUrl: ""
            },
            factory: {
                dashboardUrl: "https://app.factory.ai/settings/billing",
                statusPageUrl: "https://status.factory.ai",
                statusLinkUrl: "",
                subscriptionDashboardUrl: ""
            },
            gemini: {
                dashboardUrl: "https://gemini.google.com",
                statusPageUrl: "",
                statusLinkUrl: "https://www.google.com/appsstatus/dashboard/products/npdyhgECDJ6tB66MxXyo/history",
                subscriptionDashboardUrl: ""
            },
            antigravity: {
                dashboardUrl: "",
                statusPageUrl: "",
                statusLinkUrl: "https://www.google.com/appsstatus/dashboard/products/npdyhgECDJ6tB66MxXyo/history",
                subscriptionDashboardUrl: ""
            },
            copilot: {
                dashboardUrl: "https://github.com/settings/copilot",
                statusPageUrl: "https://www.githubstatus.com/",
                statusLinkUrl: "",
                subscriptionDashboardUrl: ""
            },
            zai: {
                dashboardUrl: "https://z.ai/manage-apikey/subscription",
                statusPageUrl: "",
                statusLinkUrl: "",
                subscriptionDashboardUrl: ""
            },
            minimax: {
                dashboardUrl: "https://platform.minimax.io/user-center/payment/coding-plan?cycle_type=3",
                statusPageUrl: "",
                statusLinkUrl: "",
                subscriptionDashboardUrl: ""
            },
            kimi: {
                dashboardUrl: "https://www.kimi.com/code/console",
                statusPageUrl: "",
                statusLinkUrl: "",
                subscriptionDashboardUrl: ""
            },
            kimik2: {
                dashboardUrl: "https://kimi-k2.ai/my-credits",
                statusPageUrl: "",
                statusLinkUrl: "",
                subscriptionDashboardUrl: ""
            },
            kiro: {
                dashboardUrl: "https://app.kiro.dev/account/usage",
                statusPageUrl: "",
                statusLinkUrl: "https://health.aws.amazon.com/health/status",
                subscriptionDashboardUrl: ""
            },
            vertexai: {
                dashboardUrl: "https://console.cloud.google.com/vertex-ai",
                statusPageUrl: "",
                statusLinkUrl: "https://status.cloud.google.com",
                subscriptionDashboardUrl: ""
            },
            augment: {
                dashboardUrl: "https://app.augmentcode.com/account/subscription",
                statusPageUrl: "",
                statusLinkUrl: "",
                subscriptionDashboardUrl: ""
            },
            amp: {
                dashboardUrl: "https://ampcode.com/settings",
                statusPageUrl: "",
                statusLinkUrl: "",
                subscriptionDashboardUrl: ""
            },
            opencode: {
                dashboardUrl: "https://opencode.ai",
                statusPageUrl: "",
                statusLinkUrl: "",
                subscriptionDashboardUrl: ""
            },
            warp: {
                dashboardUrl: "https://app.warp.dev/settings/account",
                statusPageUrl: "",
                statusLinkUrl: "",
                subscriptionDashboardUrl: ""
            }
        };

        return metadataByProvider[normalizedProvider(providerId)] || {
            dashboardUrl: "",
            statusPageUrl: "",
            statusLinkUrl: "",
            subscriptionDashboardUrl: ""
        };
    }

    function providerSupportsLogin(providerId) {
        var provider = normalizedProvider(providerId);
        return provider === "codex"
            || provider === "claude"
            || provider === "cursor"
            || provider === "factory"
            || provider === "gemini"
            || provider === "vertexai"
            || provider === "copilot";
    }

    function providerHasAccount(entry) {
        if (!entry || !entry.identity) {
            return false;
        }

        return nonEmptyString(entry.identity.accountEmail).length > 0;
    }

    function isSubscriptionPlan(loginMethod) {
        var method = nonEmptyString(loginMethod).toLowerCase();
        if (method.length === 0) {
            return false;
        }

        return method.indexOf("max") >= 0
            || method.indexOf("pro") >= 0
            || method.indexOf("ultra") >= 0
            || method.indexOf("team") >= 0;
    }

    function accountActionLabel() {
        var provider = preferredProviderId();
        if (!providerSupportsLogin(provider)) {
            return "";
        }

        return providerHasAccount(preferredProviderEntry()) ? "Switch Account..." : "Add Account...";
    }

    function dashboardUrlFor(providerId, entry) {
        var provider = normalizedProvider(providerId);
        var metadata = providerMetadata(provider);
        var dashboardUrl = nonEmptyString(metadata.dashboardUrl);

        if (provider === "claude") {
            var loginMethod = entry && entry.identity ? entry.identity.loginMethod : "";
            if (isSubscriptionPlan(loginMethod)) {
                var subscriptionUrl = nonEmptyString(metadata.subscriptionDashboardUrl);
                if (subscriptionUrl.length > 0) {
                    dashboardUrl = subscriptionUrl;
                }
            }
        }

        return dashboardUrl;
    }

    function statusPageUrlFor(providerId, entry) {
        var fromEntry = entry && entry.status ? nonEmptyString(entry.status.url) : "";
        if (fromEntry.length > 0) {
            return fromEntry;
        }

        var metadata = providerMetadata(providerId);
        var statusPageUrl = nonEmptyString(metadata.statusPageUrl);
        if (statusPageUrl.length > 0) {
            return statusPageUrl;
        }

        return nonEmptyString(metadata.statusLinkUrl);
    }

    function hasAccountAction() {
        return providerSupportsLogin(preferredProviderId());
    }

    function hasUsageDashboardAction() {
        return dashboardUrlFor(preferredProviderId(), preferredProviderEntry()).length > 0;
    }

    function hasStatusPageAction() {
        return statusPageUrlFor(preferredProviderId(), preferredProviderEntry()).length > 0;
    }

    function shellSingleQuoted(value) {
        return "'" + String(value).replace(/'/g, "'\"'\"'") + "'";
    }

    function openExternalUrl(url) {
        var external = nonEmptyString(url);
        if (external.length === 0) {
            return false;
        }
        return Qt.openUrlExternally(external);
    }

    function launchTerminalCommand(command) {
        var loginCommand = nonEmptyString(command);
        if (loginCommand.length === 0 || isShuttingDown) {
            return;
        }

        var quoted = shellSingleQuoted(loginCommand);
        var launch = "x-terminal-emulator -e sh -lc " + quoted
            + " || konsole -e sh -lc " + quoted
            + " || gnome-terminal -- sh -lc " + quoted
            + " || xterm -e sh -lc " + quoted
            + " || sh -lc " + quoted;
        actionExecutor.exec("sh -lc " + shellSingleQuoted(launch));
    }

    function loginActionForProvider(providerId) {
        var provider = normalizedProvider(providerId);
        if (provider === "codex") {
            return {kind: "terminal", command: "codex login"};
        }
        if (provider === "claude") {
            return {kind: "terminal", command: "claude /login"};
        }
        if (provider === "cursor") {
            return {kind: "url", url: "https://cursor.com/dashboard"};
        }
        if (provider === "factory") {
            return {kind: "url", url: "https://app.factory.ai"};
        }
        if (provider === "gemini") {
            return {kind: "terminal", command: "gemini"};
        }
        if (provider === "vertexai") {
            return {
                kind: "terminal",
                command: "gcloud auth application-default login --scopes=openid,https://www.googleapis.com/auth/userinfo.email,https://www.googleapis.com/auth/cloud-platform"
            };
        }
        if (provider === "copilot") {
            return {kind: "url", url: "https://github.com/login/device"};
        }

        return null;
    }

    function collapsePopup() {
        if (typeof root.expanded === "boolean") {
            root.expanded = false;
            return;
        }

        Plasmoid.expanded = false;
    }

    function runAccountAction() {
        var action = loginActionForProvider(preferredProviderId());
        if (!action) {
            return;
        }

        if (action.kind === "terminal") {
            launchTerminalCommand(action.command);
            collapsePopup();
            return;
        }

        if (action.kind === "url" && openExternalUrl(action.url)) {
            collapsePopup();
        }
    }

    function openUsageDashboard() {
        var provider = preferredProviderId();
        var url = dashboardUrlFor(provider, preferredProviderEntry());
        if (openExternalUrl(url)) {
            collapsePopup();
        }
    }

    function openStatusPage() {
        var provider = preferredProviderId();
        var url = statusPageUrlFor(provider, preferredProviderEntry());
        if (openExternalUrl(url)) {
            collapsePopup();
        }
    }

    function disconnectDataSource(dataSource) {
        if (!dataSource || !dataSource.connectedSources) {
            return;
        }

        var sources = [];
        for (var i = 0; i < dataSource.connectedSources.length; ++i) {
            sources.push(dataSource.connectedSources[i]);
        }

        for (var j = 0; j < sources.length; ++j) {
            dataSource.disconnectSource(sources[j]);
        }
    }

    function disconnectAllSources() {
        disconnectDataSource(executable);
        disconnectDataSource(actionExecutor);
    }

    function refreshSnapshot() {
        if (isShuttingDown) {
            return;
        }

        var configured = "";
        if (Plasmoid.configuration && Plasmoid.configuration.serviceCommand) {
            configured = String(Plasmoid.configuration.serviceCommand).trim();
        }

        var command = configured.length > 0
            ? configured
            : "codexbar-service snapshot --from-codexbar-cli --provider all --status";
        var wrapped = "export PATH=\"$HOME/.local/bin:$PATH\"; " + command;
        executable.exec("sh -lc " + shellSingleQuoted(wrapped));
    }

    function toggleExpanded() {
        if (isShuttingDown) {
            return;
        }
        Plasmoid.expanded = !Plasmoid.expanded;
    }

    Plasmoid.compactRepresentation: Item {
        readonly property int visualWidth: root.iconWidth
        readonly property int visualHeight: root.iconHeight

        implicitWidth: visualWidth
        implicitHeight: visualHeight
        Layout.minimumWidth: visualWidth
        Layout.preferredWidth: visualWidth
        Layout.maximumWidth: visualWidth
        Layout.minimumHeight: visualHeight
        Layout.preferredHeight: visualHeight
        Layout.maximumHeight: visualHeight
        clip: true

        readonly property var entry: root.currentEntry()
        readonly property real visualScale: {
            var widthScale = width > 0 ? width / Math.max(1, visualWidth) : 1;
            var heightScale = height > 0 ? height / Math.max(1, visualHeight) : 1;
            return Math.min(1, widthScale, heightScale);
        }

        Item {
            id: compactVisual
            width: visualWidth
            height: visualHeight
            anchors.centerIn: parent
            scale: visualScale
            transformOrigin: Item.Center
            layer.enabled: true
            layer.smooth: true
            readonly property real horizontalPadding: Math.max(2, Math.round(height * 0.14))
            readonly property real topPadding: Math.max(2, Math.round(height * 0.18))
            readonly property real rowSpacing: Math.max(1, Math.round(height * 0.1))
            readonly property real labelWidth: Math.max(11, Math.round(width * 0.19))
            readonly property real tracksLeft: horizontalPadding + labelWidth + Math.max(3, Math.round(height * 0.1))
            readonly property real tracksWidth: Math.max(8, width - tracksLeft - horizontalPadding)
            readonly property real primaryTrackHeight: Math.max(4, Math.round(height * 0.24))
            readonly property real secondaryTrackHeight: Math.max(3, Math.round(height * 0.16))

            Rectangle {
                anchors.fill: parent
                radius: height / 2
                color: root.applyAlpha(root.useDarkTheme ? "#31364D" : "#D6D2F0", root.cardOpacity)
                border.width: 1
                border.color: root.cardBorder
            }

            Item {
                id: compactProviderLabel
                x: compactVisual.horizontalPadding
                width: compactVisual.labelWidth
                height: Math.max(8, compactVisual.height - (compactVisual.topPadding * 2))
                anchors.verticalCenter: parent.verticalCenter

                Column {
                    anchors.centerIn: parent
                    spacing: 0

                    Text {
                        anchors.horizontalCenter: parent.horizontalCenter
                        text: "Codex"
                        color: root.textStrong
                        font.bold: true
                        font.pixelSize: Math.max(5, Math.round(compactVisual.height * 0.16))
                    }

                    Text {
                        anchors.horizontalCenter: parent.horizontalCenter
                        text: "Bar"
                        color: root.textStrong
                        font.bold: true
                        font.pixelSize: Math.max(5, Math.round(compactVisual.height * 0.16))
                    }
                }
            }

            Item {
                x: compactVisual.tracksLeft
                y: compactVisual.topPadding
                width: compactVisual.tracksWidth
                height: compactVisual.primaryTrackHeight + compactVisual.rowSpacing + compactVisual.secondaryTrackHeight

                Rectangle {
                    id: compactPrimaryTrack
                    x: 0
                    y: 0
                    width: parent.width
                    height: compactVisual.primaryTrackHeight
                    color: root.trackColor
                    radius: height / 2

                    Rectangle {
                        anchors.left: parent.left
                        anchors.verticalCenter: parent.verticalCenter
                        width: parent.width * (root.usedPercent(entry ? entry.primary : null) / 100.0)
                        height: parent.height
                        radius: height / 2
                        color: root.usageColor(entry ? entry.primary : null)
                    }
                }

                Rectangle {
                    x: 0
                    y: compactPrimaryTrack.y + compactPrimaryTrack.height + compactVisual.rowSpacing
                    width: parent.width
                    height: compactVisual.secondaryTrackHeight
                    color: root.trackColor
                    radius: height / 2

                    Rectangle {
                        anchors.left: parent.left
                        anchors.verticalCenter: parent.verticalCenter
                        width: parent.width * (root.usedPercent(entry ? entry.secondary : null) / 100.0)
                        height: parent.height
                        radius: height / 2
                        color: root.usageColor(entry ? entry.secondary : null)
                    }
                }
            }
        }

        MouseArea {
            anchors.fill: parent
            acceptedButtons: Qt.LeftButton
            onClicked: root.toggleExpanded()
        }
    }

    Plasmoid.fullRepresentation: Item {
        implicitWidth: Kirigami.Units.gridUnit * 22
        implicitHeight: Kirigami.Units.gridUnit * 24

        readonly property var entry: root.displayEntry()
        readonly property bool noDataAvailable: !root.hasAnyUsageData(entry)
        readonly property real reviewUsed: root.codeReviewUsedPercent(entry)

        Rectangle {
            anchors.fill: parent
            radius: Kirigami.Units.largeSpacing
            color: root.applyAlpha(root.cardBackground, root.cardOpacity)
            border.color: root.cardBorder
            border.width: 1
        }

        Flickable {
            id: popupScroll
            anchors.fill: parent
            anchors.margins: Kirigami.Units.largeSpacing
            clip: true
            contentWidth: width
            contentHeight: contentColumn.implicitHeight
            boundsBehavior: Flickable.StopAtBounds
            flickableDirection: Flickable.VerticalFlick

            ColumnLayout {
                id: contentColumn
                width: popupScroll.width
                spacing: Kirigami.Units.smallSpacing

                Row {
                spacing: Kirigami.Units.smallSpacing
                Layout.fillWidth: true

                Repeater {
                    model: root.tabModel()

                    delegate: Rectangle {
                        required property var modelData

                        readonly property string providerId: root.normalizedProvider(modelData.provider)
                        readonly property bool active: root.preferredProviderId() === providerId
                        readonly property real weeklyUsed: root.weeklyUsageForProvider(providerId)

                        radius: Kirigami.Units.smallSpacing
                        color: active ? root.tabActive : root.tabInactive
                        border.color: active ? "#4F84EA" : root.cardBorder
                        border.width: 1
                        implicitHeight: Kirigami.Units.gridUnit * 2.2
                        implicitWidth: Math.max(Kirigami.Units.gridUnit * 3.1, tabLabel.implicitWidth + Kirigami.Units.largeSpacing)

                        Column {
                            anchors.fill: parent
                            anchors.margins: Math.max(2, Kirigami.Units.smallSpacing / 1.4)
                            spacing: Math.max(1, Kirigami.Units.smallSpacing / 2)

                            Item {
                                width: parent.width
                                height: Kirigami.Units.gridUnit * 0.72

                                Canvas {
                                    id: tabIconCanvas
                                    anchors.centerIn: parent
                                    width: parent.height
                                    height: parent.height
                                    antialiasing: true
                                    renderTarget: Canvas.Image

                                    onPaint: {
                                        var ctx = getContext("2d");
                                        root.paintProviderTabIcon(ctx, providerId, width, height, active);
                                    }

                                    onWidthChanged: requestPaint()
                                    onHeightChanged: requestPaint()
                                    Component.onCompleted: requestPaint()
                                }
                            }

                            Text {
                                id: tabLabel
                                width: parent.width
                                horizontalAlignment: Text.AlignHCenter
                                text: String(modelData.label || "")
                                color: active ? "#FFFFFF" : root.textStrong
                                font.bold: active
                                font.pixelSize: Kirigami.Units.gridUnit * 0.42
                                elide: Text.ElideRight
                            }

                            Rectangle {
                                width: parent.width
                                height: Math.max(2, Kirigami.Units.smallSpacing * 0.62)
                                radius: height / 2
                                color: root.applyAlpha(active ? "#FFFFFF" : root.trackColor, active ? 0.32 : 1.0)

                                Rectangle {
                                    anchors.left: parent.left
                                    anchors.verticalCenter: parent.verticalCenter
                                    height: parent.height
                                    width: parent.width * (weeklyUsed / 100.0)
                                    radius: height / 2
                                    color: root.weeklyUsageColorForProvider(providerId)
                                }
                            }
                        }

                        MouseArea {
                            anchors.fill: parent
                            acceptedButtons: Qt.LeftButton
                            onClicked: root.selectedProvider = providerId
                        }

                        onActiveChanged: tabIconCanvas.requestPaint()
                        onProviderIdChanged: tabIconCanvas.requestPaint()

                        Connections {
                            target: root
                            function onUseDarkThemeChanged() {
                                tabIconCanvas.requestPaint();
                            }
                        }
                    }
                }
            }

                Rectangle {
                Layout.fillWidth: true
                implicitHeight: 1
                color: root.sectionDivider
                opacity: 0.8
                }

                Text {
                visible: root.lastError.length > 0
                text: root.lastError
                color: "#B13C4F"
                font.pixelSize: Kirigami.Units.gridUnit * 0.56
                wrapMode: Text.Wrap
                Layout.fillWidth: true
                }

                Text {
                visible: noDataAvailable
                text: i18n("No data found for this agent.")
                color: "#B13C4F"
                font.pixelSize: Kirigami.Units.gridUnit * 0.5
                wrapMode: Text.Wrap
                Layout.fillWidth: true
                }

                ColumnLayout {
                spacing: Kirigami.Units.smallSpacing
                Layout.fillWidth: true

                RowLayout {
                    Layout.fillWidth: true

                    Text {
                        text: String(entry.provider || "").charAt(0).toUpperCase() + String(entry.provider || "").slice(1)
                        color: root.textStrong
                        font.bold: true
                        font.pixelSize: Kirigami.Units.gridUnit * 0.9
                        Layout.fillWidth: true
                    }

                    Rectangle {
                        radius: Kirigami.Units.smallSpacing
                        color: root.badgeBackground
                        border.color: root.cardBorder
                        border.width: 1
                        implicitHeight: Kirigami.Units.gridUnit * 1.2
                        implicitWidth: sourceText.implicitWidth + Kirigami.Units.smallSpacing * 2

                        Text {
                            id: sourceText
                            anchors.centerIn: parent
                            text: root.sourceLabel(entry.source)
                            color: root.textMuted
                            font.pixelSize: Kirigami.Units.gridUnit * 0.46
                            font.bold: true
                        }
                    }
                }

                Text {
                    text: root.updatedLabel(entry)
                    color: root.textMuted
                    font.pixelSize: Kirigami.Units.gridUnit * 0.52
                    Layout.fillWidth: true
                }

                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: 1
                    color: root.sectionDivider
                    opacity: 0.75
                }

                Text {
                    text: "Session"
                    color: root.textStrong
                    font.bold: true
                    font.pixelSize: Kirigami.Units.gridUnit * 0.66
                }

                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: Kirigami.Units.smallSpacing * 1.6
                    radius: height / 2
                    color: root.trackColor

                    Rectangle {
                        anchors.left: parent.left
                        anchors.verticalCenter: parent.verticalCenter
                        height: parent.height
                        width: parent.width * (root.usedPercent(entry.primary) / 100.0)
                        radius: height / 2
                        color: root.usageColor(entry.primary)
                    }
                }

                RowLayout {
                    Layout.fillWidth: true

                    Text {
                        text: root.usageText(entry.primary)
                        color: root.textStrong
                        font.pixelSize: Kirigami.Units.gridUnit * 0.52
                        font.bold: true
                        Layout.fillWidth: true
                    }

                    Text {
                        text: root.resetCountdown(entry.primary)
                        color: root.textMuted
                        font.pixelSize: Kirigami.Units.gridUnit * 0.52
                    }
                }

                Text {
                    text: "Weekly"
                    color: root.textStrong
                    font.bold: true
                    font.pixelSize: Kirigami.Units.gridUnit * 0.66
                }

                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: Kirigami.Units.smallSpacing * 1.25
                    radius: height / 2
                    color: root.trackColor

                    Rectangle {
                        anchors.left: parent.left
                        anchors.verticalCenter: parent.verticalCenter
                        height: parent.height
                        width: parent.width * (root.usedPercent(entry.secondary) / 100.0)
                        radius: height / 2
                        color: root.usageColor(entry.secondary)
                    }
                }

                RowLayout {
                    Layout.fillWidth: true

                    Text {
                        text: root.usageText(entry.secondary)
                        color: root.textStrong
                        font.pixelSize: Kirigami.Units.gridUnit * 0.52
                        font.bold: true
                        Layout.fillWidth: true
                    }

                    Text {
                        text: root.resetCountdown(entry.secondary)
                        color: root.textMuted
                        font.pixelSize: Kirigami.Units.gridUnit * 0.52
                    }
                }

                Text {
                    visible: root.isClaudeProvider(entry.provider)
                    text: "Sonnet"
                    color: root.textStrong
                    font.bold: true
                    font.pixelSize: Kirigami.Units.gridUnit * 0.66
                }

                Rectangle {
                    visible: root.isClaudeProvider(entry.provider)
                    Layout.fillWidth: true
                    implicitHeight: Kirigami.Units.smallSpacing * 1.2
                    radius: height / 2
                    color: root.trackColor

                    Rectangle {
                        anchors.left: parent.left
                        anchors.verticalCenter: parent.verticalCenter
                        height: parent.height
                        width: parent.width * (root.usedPercent(entry.tertiary) / 100.0)
                        radius: height / 2
                        color: root.usageColor(entry.tertiary)
                    }
                }

                Text {
                    visible: root.isClaudeProvider(entry.provider)
                    text: root.usageText(entry.tertiary)
                    color: root.textStrong
                    font.pixelSize: Kirigami.Units.gridUnit * 0.52
                    font.bold: true
                    Layout.fillWidth: true
                }

                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: 1
                    color: root.sectionDivider
                    opacity: 0.75
                }

                Text {
                    text: "Extra usage"
                    color: root.textStrong
                    font.bold: true
                    font.pixelSize: Kirigami.Units.gridUnit * 0.66
                }

                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: Kirigami.Units.smallSpacing * 1.15
                    radius: height / 2
                    color: root.trackColor

                    Rectangle {
                        anchors.left: parent.left
                        anchors.verticalCenter: parent.verticalCenter
                        height: parent.height
                        width: reviewUsed >= 0 ? parent.width * (reviewUsed / 100.0) : 0
                        radius: height / 2
                        color: "#5E89E8"
                    }
                }

                RowLayout {
                    Layout.fillWidth: true

                    Text {
                        text: root.creditsText(entry)
                        color: root.textStrong
                        font.pixelSize: Kirigami.Units.gridUnit * 0.5
                        font.bold: true
                        Layout.fillWidth: true
                    }

                    Text {
                        text: root.codeReviewUsedText(entry)
                        color: root.textMuted
                        font.pixelSize: Kirigami.Units.gridUnit * 0.5
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: 1
                    color: root.sectionDivider
                    opacity: 0.75
                }

                Text {
                    text: "Cost"
                    color: root.textStrong
                    font.bold: true
                    font.pixelSize: Kirigami.Units.gridUnit * 0.66
                }

                Text {
                    text: "Cost metrics are not available in the KDE preview yet."
                    color: root.textMuted
                    font.pixelSize: Kirigami.Units.gridUnit * 0.5
                    wrapMode: Text.Wrap
                    Layout.fillWidth: true
                }

                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: 1
                    color: root.sectionDivider
                    opacity: 0.75
                }

                Item {
                    visible: root.hasAccountAction()
                    Layout.fillWidth: true
                    implicitHeight: accountActionText.implicitHeight + Kirigami.Units.smallSpacing

                    Text {
                        id: accountActionText
                        anchors.left: parent.left
                        anchors.verticalCenter: parent.verticalCenter
                        text: root.accountActionLabel()
                        color: root.textStrong
                        opacity: accountActionMouseArea.containsMouse ? 0.78 : 1.0
                        font.pixelSize: Kirigami.Units.gridUnit * 0.66
                        font.bold: true
                    }

                    MouseArea {
                        id: accountActionMouseArea
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        enabled: root.hasAccountAction()
                        onClicked: root.runAccountAction()
                    }
                }

                Item {
                    visible: root.hasUsageDashboardAction()
                    Layout.fillWidth: true
                    implicitHeight: dashboardActionText.implicitHeight + Kirigami.Units.smallSpacing

                    Text {
                        id: dashboardActionText
                        anchors.left: parent.left
                        anchors.verticalCenter: parent.verticalCenter
                        text: "Usage Dashboard"
                        color: root.textStrong
                        opacity: dashboardActionMouseArea.containsMouse ? 0.78 : 1.0
                        font.pixelSize: Kirigami.Units.gridUnit * 0.66
                        font.bold: true
                    }

                    MouseArea {
                        id: dashboardActionMouseArea
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        enabled: root.hasUsageDashboardAction()
                        onClicked: root.openUsageDashboard()
                    }
                }

                Item {
                    visible: root.hasStatusPageAction()
                    Layout.fillWidth: true
                    implicitHeight: statusActionText.implicitHeight + Kirigami.Units.smallSpacing

                    Text {
                        id: statusActionText
                        anchors.left: parent.left
                        anchors.verticalCenter: parent.verticalCenter
                        text: "Status Page"
                        color: root.textStrong
                        opacity: statusActionMouseArea.containsMouse ? 0.78 : 1.0
                        font.pixelSize: Kirigami.Units.gridUnit * 0.66
                        font.bold: true
                    }

                    MouseArea {
                        id: statusActionMouseArea
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        enabled: root.hasStatusPageAction()
                        onClicked: root.openStatusPage()
                    }
                }
                }
            }
        }
    }

    PlasmaCore.DataSource {
        id: executable
        engine: "executable"
        connectedSources: []

        function exec(command) {
            if (root.isShuttingDown) {
                return;
            }
            disconnectSource(command);
            connectSource(command);
        }

        onNewData: function (sourceName, data) {
            if (root.isShuttingDown) {
                disconnectSource(sourceName);
                return;
            }

            var stdout = data.stdout ? data.stdout.toString().trim() : "";
            var stderr = data.stderr ? data.stderr.toString().trim() : "";

            if (!stdout) {
                root.snapshot = ({generatedAt: "", entries: []});
                root.lastError = stderr.length > 0 ? stderr : i18n("No data from service command");
                disconnectSource(sourceName);
                return;
            }

            try {
                var parsed = JSON.parse(stdout);
                root.snapshot = parsed;
                root.lastError = "";
            } catch (error) {
                root.snapshot = ({generatedAt: "", entries: []});
                root.lastError = i18n("Invalid snapshot JSON");
            }

            disconnectSource(sourceName);
        }
    }

    PlasmaCore.DataSource {
        id: actionExecutor
        engine: "executable"
        connectedSources: []

        function exec(command) {
            if (root.isShuttingDown) {
                return;
            }
            disconnectSource(command);
            connectSource(command);
        }

        onNewData: function (sourceName, data) {
            var _ = data;
            disconnectSource(sourceName);
        }
    }

    Timer {
        id: refreshTimer
        interval: {
            var configured = Number(
                Plasmoid.configuration && Plasmoid.configuration.refreshSeconds !== undefined
                ? Plasmoid.configuration.refreshSeconds
                : 60);
            if (!isFinite(configured)) {
                configured = 60;
            }
            return Math.max(15, Math.round(configured)) * 1000;
        }
        repeat: true
        running: false
        onTriggered: root.refreshSnapshot()
    }

    Component.onCompleted: {
        refreshTimer.start();
        root.refreshSnapshot();
    }

    Component.onDestruction: {
        root.isShuttingDown = true;
        refreshTimer.stop();
        root.disconnectAllSources();
    }
}
