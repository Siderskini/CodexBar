import QtQuick 2.15
import QtQuick.Controls 2.15 as QQC2
import org.kde.kirigami 2.20 as Kirigami

Kirigami.FormLayout {
    property alias cfg_serviceCommand: serviceCommandField.text
    property alias cfg_refreshSeconds: refreshSecondsField.value

    QQC2.TextField {
        id: serviceCommandField
        Kirigami.FormData.label: i18n("Service command:")
        placeholderText: "codexbar-service snapshot --from-codexbar-cli --provider all --status"
    }

    QQC2.SpinBox {
        id: refreshSecondsField
        Kirigami.FormData.label: i18n("Refresh (seconds):")
        from: 15
        to: 3600
        stepSize: 15
    }

}
