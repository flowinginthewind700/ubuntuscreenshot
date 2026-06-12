use gpui::actions;

actions!(
    screenshot4ubuntu,
    [
        StartCapture,
        CancelCapture,
        ConfirmCapture,
        SaveCapture,
        CopyCapture,
        UndoAnnotation,
        CopyPreview,
        SavePreview,
        ClosePreview,
        Quit,
    ]
);
