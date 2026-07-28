// Break: camelCase identifiers in a tree that is 88% flat lower-case.
function calculateWindowOffset(const senderWidget: twidget;
                               const requestedShift: pointty): pointty;
var
 currentBounds: rectty;
 maximumWidth: integer;
begin
 currentBounds:= senderWidget.widgetrect;
 maximumWidth:= currentBounds.cx;
 result.x:= requestedShift.x;
 result.y:= requestedShift.y;
 if result.x > maximumWidth then begin
  result.x:= maximumWidth;
 end;
end;
