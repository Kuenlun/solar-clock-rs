# solar-clock-rs
Solar Clock using SPA

This algorithm converts a local real-world datetime (which may contain DST jumps) into a continuous "Solar Clock" time where solar events always occur at fixed reference hours.

**Key Definition:**
*   **Solar Reference Timezone**: A fixed timezone (e.g., UTC+1). No DST.
*   **Solar Reference Events**:
    *   Sunrise: 08:00
    *   Noon: 14:00
    *   Sunset: 20:00
    *(These target times are arbitrary and customizable)*

## Algorithm Steps

### 1. Normalize Input
Take the User Input Datetime (Local Time) and convert it to **UTC** (Coordinated Universal Time).
*   *Why*: UTC is a continuous physical time scale without political time shifts (Daylight Saving Time). All calculations must occur in this continuous domain.

### 2. Calculate Reference Points (UTC)
For the current day (and adjacent days to ensure continuity), calculate the *actual physical moments* of the solar events in UTC based on geographic coordinates:
*   $T_{Real\_Sunrise\_UTC}$
*   $T_{Real\_Noon\_UTC}$
*   $T_{Real\_Sunset\_UTC}$

### 3. Define Target Points (Solar Time)
Map each physical event to its corresponding target time in the Fixed Solar Reference Timezone:
*   $T_{Target\_Sunrise} = Date + 08:00$ (Fixed Timezone)
*   $T_{Target\_Noon}    = Date + 14:00$ (Fixed Timezone)
*   $T_{Target\_Sunset}  = Date + 20:00$ (Fixed Timezone)

### 4. Build Interpolation Model
Create a set of pairs mapping physical time to the time difference required to reach the target solar time:
*   $X = T_{Real\_Event\_UTC}$
*   $Y = (T_{Target\_Event} - T_{Real\_Event\_UTC})$
*   *Why difference?*: Interpolating the delta instead of absolute timestamps avoids floating-point precision errors with large values and isolates the solar correction curve for better accuracy.

Use **PCHIP Interpolation** (Piecewise Cubic Hermite Interpolating Polynomial) on these points.
*   *Why PCHIP*: It ensures the curve is monotonic and continuous, preventing overshoots between control points (unlike cubic splines).

### 5. Calculate Output
1.  Identify the user input's timestamp in UTC: $t_{input}$.
2.  Calculate the time offset using the model: $\Delta t = PCHIP(t_{input})$.
3.  Compute Solar Time Timestamp: $t_{solar} = t_{input} + \Delta t$.
4.  Formatting: Convert $t_{solar}$ to the Fixed Solar Reference Timezone.

## Result
The output is a continuous time value where sunrise, noon, and sunset align with the user's desired schedule, completely unaffected by local DST transitions.
