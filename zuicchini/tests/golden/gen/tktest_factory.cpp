// Factory for emTestPanel::TkTest — compiled separately against the scaffold
// header (emTkTestPanel.h) to avoid conflict with the real emTestPanel.h.

#include <emTkTest/emTkTestPanel.h>

emPanel* create_tktest(emPanel::ParentArg parent, const emString& name) {
    return new emTestPanel::TkTest(parent, name);
}
