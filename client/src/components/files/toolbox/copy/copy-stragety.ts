import { ClipboardOperation } from "@/services/files/clipboard-service";

enum CopyButtonIcon {
    Cut = "drive_file_move",
    Copy = "file_copy",
}

enum CopyButtonTitle {
    Cut = "Cut",
    Copy = "Copy",
}

interface CopyStrategy {
    buttonTitle: string;
    buttonIcon: string;
}

class CopyStrategyImpl implements CopyStrategy {
    constructor(public buttonTitle: string, public buttonIcon: string) {}
}

class CopyStrategyChooser {
    public static get(operation: ClipboardOperation): CopyStrategy {
        switch (operation) {
            case ClipboardOperation.Cut:
                return new CopyStrategyImpl(CopyButtonTitle.Cut, CopyButtonIcon.Cut);
            case ClipboardOperation.Copy:
                return new CopyStrategyImpl(CopyButtonTitle.Copy, CopyButtonIcon.Copy);
        }
    }
}

export { CopyStrategyChooser, type CopyStrategy };
